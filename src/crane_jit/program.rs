use crate::parser::{parse, Node};
use crate::{read, write, INIT_MEMORY_SIZE};
use cranelift::codegen::control::ControlPlane;
use cranelift::codegen::ir::{Function, UserFuncName};
use cranelift::codegen::{verify_function, Context};
use cranelift::prelude::isa::CallConv;
use cranelift::prelude::types::I8;
use cranelift::prelude::*;

pub struct Program {
    bytes: Vec<u8>,
}

impl Program {
    pub fn new(source: &str) -> Result<Program, String> {
        let mut builder = settings::builder();
        builder.set("opt_level", "speed").unwrap();
        let flags = settings::Flags::new(builder);

        let isa_builder = cranelift_native::builder()
            .unwrap_or_else(|msg| panic!("host machine is not a supported target: {}", msg));
        let isa = isa_builder.finish(flags).unwrap();

        let pointer_type = isa.pointer_type();

        // receive memory address as parameter, and return pointer to io::Error
        let mut sig = Signature::new(CallConv::SystemV);
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(pointer_type));

        let mut func = Function::with_name_signature(UserFuncName::user(0, 0), sig);

        let mut func_ctx = FunctionBuilderContext::new();
        let mut builder = FunctionBuilder::new(&mut func, &mut func_ctx);

        // create a variable `pointer` (offset from memory address)
        let pointer = Variable::new(0);
        builder.declare_var(pointer, pointer_type);

        let block = builder.create_block();
        builder.seal_block(block);

        builder.append_block_params_for_function_params(block);
        builder.switch_to_block(block);

        let memory_address = builder.block_params(block)[0];

        // initialize pointer to 0
        let zero = builder.ins().iconst(pointer_type, 0);
        builder.def_var(pointer, zero);

        let code = parse(source)?;
        let mut loop_stack = Vec::new();
        let mem_flags = MemFlags::new();

        let (write_sig, write_address) = {
            let mut write_sig = Signature::new(CallConv::SystemV);
            write_sig.params.push(AbiParam::new(I8));
            write_sig.returns.push(AbiParam::new(pointer_type));
            let write_sig = builder.import_signature(write_sig);

            let write_address = write as *const () as i64;
            let write_address = builder.ins().iconst(pointer_type, write_address);
            (write_sig, write_address)
        };

        let (read_sig, read_address) = {
            let mut read_sig = Signature::new(CallConv::SystemV);
            read_sig.params.push(AbiParam::new(pointer_type));
            read_sig.returns.push(AbiParam::new(pointer_type));
            let read_sig = builder.import_signature(read_sig);

            let read_address = read as *const () as i64;
            let read_address = builder.ins().iconst(pointer_type, read_address);
            (read_sig, read_address)
        };

        let exit_block = builder.create_block();
        builder.append_block_param(exit_block, pointer_type);

        for c in code {
            match c {
                Node::Increment(n) => {
                    let pointer_value = builder.use_var(pointer);
                    let cell_address = builder.ins().iadd(memory_address, pointer_value);
                    let cell_value = builder.ins().load(I8, mem_flags, cell_address, 0);
                    let cell_value = builder.ins().iadd_imm(cell_value, n as i64);
                    builder.ins().store(mem_flags, cell_value, cell_address, 0);
                }
                Node::Decrement(n) => {
                    let pointer_value = builder.use_var(pointer);
                    let cell_address = builder.ins().iadd(memory_address, pointer_value);
                    let cell_value = builder.ins().load(I8, mem_flags, cell_address, 0);
                    let cell_value = builder.ins().iadd_imm(cell_value, -(n as i64));
                    builder.ins().store(mem_flags, cell_value, cell_address, 0);
                }
                Node::Prev(n) => {
                    let pointer_value = builder.use_var(pointer);
                    let pointer_value = builder.ins().iadd_imm(pointer_value, -(n as i64));
                    builder.def_var(pointer, pointer_value);
                }
                Node::Next(n) => {
                    let pointer_value = builder.use_var(pointer);
                    let pointer_value = builder.ins().iadd_imm(pointer_value, n as i64);
                    builder.def_var(pointer, pointer_value);
                }
                Node::Write => {
                    let pointer_value = builder.use_var(pointer);
                    let cell_address = builder.ins().iadd(memory_address, pointer_value);
                    let cell_value = builder.ins().load(I8, mem_flags, cell_address, 0);

                    let inst = builder
                        .ins()
                        .call_indirect(write_sig, write_address, &[cell_value]);
                    let result = builder.inst_results(inst)[0];

                    let after_block = builder.create_block();

                    builder
                        .ins()
                        .brif(result, exit_block, &[result], after_block, &[]);

                    builder.seal_block(after_block);
                    builder.switch_to_block(after_block);
                }
                Node::Read => {
                    let pointer_value = builder.use_var(pointer);
                    let cell_address = builder.ins().iadd(memory_address, pointer_value);

                    let inst = builder
                        .ins()
                        .call_indirect(read_sig, read_address, &[cell_address]);
                    let result = builder.inst_results(inst)[0];

                    let after_block = builder.create_block();

                    builder
                        .ins()
                        .brif(result, exit_block, &[result], after_block, &[]);

                    builder.seal_block(after_block);
                    builder.switch_to_block(after_block);
                }
                Node::LoopBegin => {
                    let inner_block = builder.create_block();
                    let after_block = builder.create_block();

                    let pointer_value = builder.use_var(pointer);
                    let cell_address = builder.ins().iadd(memory_address, pointer_value);
                    let cell_value = builder.ins().load(I8, MemFlags::new(), cell_address, 0);

                    builder
                        .ins()
                        .brif(cell_value, inner_block, &[], after_block, &[]);
                    builder.switch_to_block(inner_block);

                    loop_stack.push((inner_block, after_block));
                }
                Node::LoopEnd => {
                    let (inner_block, after_block) = match loop_stack.pop() {
                        Some(x) => x,
                        None => return Err("Unclosing loop found.".to_string()),
                    };

                    let pointer_value = builder.use_var(pointer);
                    let cell_address = builder.ins().iadd(memory_address, pointer_value);
                    let cell_value = builder.ins().load(I8, mem_flags, cell_address, 0);

                    builder
                        .ins()
                        .brif(cell_value, inner_block, &[], after_block, &[]);

                    builder.seal_block(inner_block);
                    builder.seal_block(after_block);

                    builder.switch_to_block(after_block);
                }
            }
        }

        if !loop_stack.is_empty() {
            return Err("Unclosing loop found.".to_string());
        }

        builder.ins().return_(&[zero]);

        builder.switch_to_block(exit_block);
        builder.seal_block(exit_block);

        let result = builder.block_params(exit_block)[0];
        builder.ins().return_(&[result]);

        builder.finalize();

        let res = verify_function(&func, &*isa);

        if let Err(errors) = res {
            panic!("{}", errors);
        }

        let mut ctx = Context::for_function(func);
        let mut control_plane = ControlPlane::default();
        let compiled = match ctx.compile(&*isa, &mut control_plane) {
            Ok(x) => x,
            Err(err) => {
                eprintln!("error compiling: {:?}", err);
                std::process::exit(8);
            }
        };
        let bytes = compiled.code_buffer().to_vec();
        Ok(Program { bytes })
    }

    pub fn run(&mut self) -> Result<(), String> {
        let mut memory = [0; INIT_MEMORY_SIZE];

        let mut buffer = memmap2::MmapOptions::new()
            .len(self.bytes.len())
            .map_anon()
            .unwrap();
        buffer.copy_from_slice(&self.bytes);

        let buffer = buffer.make_exec().unwrap();
        unsafe {
            let func: unsafe extern "sysv64" fn(*mut u8) -> *mut std::io::Error =
                std::mem::transmute(buffer.as_ptr());
            let error = func(memory.as_mut_ptr());

            if !error.is_null() {
                return Err((*Box::from_raw(error)).to_string());
            }
        }

        Ok(())
    }
}
