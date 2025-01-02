#[derive(Debug, Copy, Clone)]
pub enum Node {
    Increment(u8),
    Decrement(u8),
    Next(usize),
    Prev(usize),
    Write,
    Read,
    LoopBegin,
    LoopEnd,
}
pub(crate) fn parse(source: &str) -> Result<Vec<Node>, String> {
    let mut code = Vec::new();
    for c in source.chars() {
        match c {
            '+' => code.push(Node::Increment(1)),
            '-' => code.push(Node::Decrement(1)),
            '>' => code.push(Node::Next(1)),
            '<' => code.push(Node::Prev(1)),
            '.' => code.push(Node::Write),
            ',' => code.push(Node::Read),
            '[' => {
                code.push(Node::LoopBegin);
            }
            ']' => {
                code.push(Node::LoopEnd);
            }
            _ => continue,
        }
    }
    code = pass_simplify(&code);
    Ok(code)
}

fn pass_simplify(code: &Vec<Node>) -> Vec<Node> {
    let mut result = Vec::new();
    for next_op in code {
        let prev_op = result.last();

        let combined = match (prev_op, next_op) {
            (Some(Node::Increment(x)), Node::Increment(y)) => {
                Some(Node::Increment(x.wrapping_add(*y)))
            }
            (Some(Node::Decrement(x)), Node::Decrement(y)) => {
                Some(Node::Decrement(x.wrapping_add(*y)))
            }
            (Some(Node::Prev(x)), Node::Prev(y)) => Some(Node::Prev(x.wrapping_add(*y))),
            (Some(Node::Next(x)), Node::Next(y)) => Some(Node::Next(x.wrapping_add(*y))),
            _ => None,
        };

        if let Some(new_op) = combined {
            result.pop();
            result.push(new_op);
        } else {
            result.push(*next_op);
        }
    }
    result
}
