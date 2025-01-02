use crate::fast_jit::code_gen;
use crate::parser::parse;
use crate::INIT_MEMORY_SIZE;
use dynasmrt::mmap::MutableBuffer;

pub struct Program {
    bytes: Vec<u8>,
}

impl Program {
    pub fn new(source: &str) -> Result<Program, String> {
        let code = parse(source)?;
        let bytes = code_gen::emit(&code)?;
        Ok(Program { bytes })
    }

    pub fn run(&self) -> Result<(), String> {
        let mut memory = [0; INIT_MEMORY_SIZE];
        let mut buffer = MutableBuffer::new(self.bytes.len()).unwrap();
        buffer.set_len(self.bytes.len());

        buffer.copy_from_slice(&self.bytes);

        let buffer = buffer.make_exec().unwrap();

        unsafe {
            let func: unsafe extern "C" fn(*mut u8) = std::mem::transmute(buffer.as_ptr());
            func(memory.as_mut_ptr());
        }

        Ok(())
    }
}
