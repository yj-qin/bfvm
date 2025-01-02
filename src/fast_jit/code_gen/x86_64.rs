use crate::parser::Node;
use crate::fast_jit::code_gen::{write, read};
use dynasmrt::{dynasm, x64::X64Relocation, DynasmApi, DynasmLabelApi, VecAssembler};

pub(crate) fn emit(code: &Vec<Node>) -> Result<Vec<u8>, String> {
    let mut bytes: VecAssembler<X64Relocation> = VecAssembler::new(0);
    let mut loop_labels = Vec::new();

    // r12 will be the address of `memory`
    // r13 will be the value of `pointer`
    // r12 is got from argument 1 in `rdi`
    // r13 is set to 0
    dynasm! { bytes
        ; .arch x64
        ; push rbp
        ; mov rbp, rsp
        ; push r12
        ; push r13
        ; mov r12, rdi
        ; xor r13, r13
    };

    for op in code {
        match op {
            Node::Increment(n) => dynasm! { bytes
                ; .arch x64
                ; add BYTE [r12 + r13], *n as i8
            },
            Node::Decrement(n) => dynasm! { bytes
                ; .arch x64
                ; sub BYTE [r12 + r13], *n as i8
            },
            Node::Next(n) => dynasm! { bytes
                ; .arch x64
                ; add r13, *n as i32
            },
            Node::Prev(n) => dynasm! { bytes
                ; .arch x64
                ; sub r13, *n as i32
            },
            Node::Write => dynasm! { bytes
                ; .arch x64
                ; mov rax, QWORD write as *const() as i64
                ; mov rdi, [r12 + r13] // buf address
                ; call rax
                ; cmp rax, 0
                ; jne ->exit
            },
            Node::Read => dynasm! { bytes
                ; .arch x64
                ; mov rax, QWORD read as *const() as i64
                ; lea rdi, [r12 + r13] // buf address
                ; call rax
                ; cmp rax, 0
                ; jne ->exit
                ; ret
            },
            Node::LoopBegin => {
                let start_label = bytes.new_dynamic_label();
                let end_label = bytes.new_dynamic_label();

                dynasm! { bytes
                    ; .arch x64
                    ; cmp BYTE [r12 + r13], 0
                    ; je =>end_label
                    ; => start_label
                }
                loop_labels.push((start_label, end_label));
            }
            Node::LoopEnd => {
                let (start_label, end_label) = match loop_labels.pop() {
                    Some(x) => x,
                    None => return Err("Unclosing loop found.".to_string()),
                };
                dynasm! { bytes
                    ; .arch x64
                    ; cmp BYTE [r12 + r13], 0
                    ; jne => start_label
                    ; => end_label
                }
            }
        }
    }

    dynasm! { bytes
        ; .arch x64
        ; xor rax, rax
        ; ->exit:
        ; pop r13
        ; pop r12
        ; pop rbp
        ; ret
    }

    bytes.finalize().map_err(|e| e.to_string())
}
