use crate::parser::{parse, Node};
use crate::INIT_MEMORY_SIZE;
use std::io::{Read, Write};
use std::{cmp, io};

pub enum OpCode {
    Increment(u8),
    Decrement(u8),
    Next(usize),
    Prev(usize),
    Write,
    Read,
    LoopBegin(usize),
    LoopEnd(usize),
}

pub(crate) struct Interpreter {
    program: Vec<OpCode>,
    memory: Vec<u8>,
    pc: usize,
    dp: usize,
}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {
            program: Vec::new(),
            memory: vec![0u8; INIT_MEMORY_SIZE],
            pc: 0,
            dp: 0,
        }
    }

    fn compile(nodes: &Vec<Node>) -> Result<Vec<OpCode>, String> {
        let mut loop_idx = Vec::new();
        let mut result = Vec::new();
        for i in 0..nodes.len() {
            let cur = &nodes[i];
            match cur {
                Node::Increment(n) => result.push(OpCode::Increment(*n)),
                Node::Decrement(n) => result.push(OpCode::Decrement(*n)),
                Node::Prev(n) => result.push(OpCode::Prev(*n)),
                Node::Next(n) => result.push(OpCode::Next(*n)),
                Node::Write => result.push(OpCode::Write),
                Node::Read => result.push(OpCode::Read),
                Node::LoopBegin => {
                    loop_idx.push(i);
                    result.push(OpCode::LoopBegin(0));
                }
                Node::LoopEnd => {
                    let idx = match loop_idx.pop() {
                        Some(idx) => idx,
                        None => return Err("Unclosing loop found.".to_string()),
                    };
                    result[idx] = OpCode::LoopBegin(i);
                    result.push(OpCode::LoopEnd(idx));
                }
            }
        }
        if !loop_idx.is_empty() {
            return Err("Unclosing loop found.".to_string());
        }
        Ok(result)
    }

    pub fn run(&mut self, source: &str) -> Result<(), String> {
        let nodes = parse(source)?;
        self.program = Self::compile(&nodes)?;
        let mut stdin = io::stdin();
        let mut stdout = io::stdout();
        loop {
            if self.pc >= self.program.len() {
                break;
            }

            if self.dp >= self.memory.len() {
                let new_len = cmp::max(self.memory.len() * 2, self.dp);
                self.memory.resize(new_len, 0);
            }

            match self.program[self.pc] {
                OpCode::Increment(n) => self.memory[self.dp] = self.memory[self.dp].wrapping_add(n),
                OpCode::Decrement(n) => self.memory[self.dp] = self.memory[self.dp].wrapping_sub(n),
                OpCode::Prev(n) => {
                    if self.dp < n {
                        eprintln!("Memory out of bounds.");
                    }
                    self.dp -= n
                }
                OpCode::Next(n) => self.dp += n,
                OpCode::Read => {
                    let mut buf = [0u8; 1];
                    if let Err(err) = stdin.read_exact(&mut buf) {
                        if err.kind() != io::ErrorKind::UnexpectedEof {
                            eprintln!("Error reading: {}.", err);
                            break;
                        }

                        buf[0] = b'\n';
                    }
                }
                OpCode::Write => {
                    if let Err(msg) = stdout.write_all(&[self.memory[self.dp]]) {
                        eprintln!("Error writing: {}.", msg);
                        break;
                    }
                }
                OpCode::LoopBegin(idx) => {
                    if self.memory[self.dp] == 0 {
                        self.pc = idx;
                    }
                }
                OpCode::LoopEnd(idx) => {
                    if self.memory[self.dp] != 0 {
                        self.pc = idx;
                    }
                }
            }

            self.pc += 1
        }

        self.reset();
        Ok(())
    }

    fn reset(&mut self) {
        self.memory = vec![0u8; INIT_MEMORY_SIZE];
        self.pc = 0;
        self.dp = 0;
    }
}
