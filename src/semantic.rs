use crate::ast::{Node, NodeIdx, NodeTag, StringTable};

pub struct Semantic {
    nodes: Vec<Node>,
    strings: StringTable,
}

impl Semantic {
    pub fn new(nodes: Vec<Node>, strings: StringTable) -> Self {
        Semantic { nodes, strings }
    }

    pub fn check(&mut self, root: NodeIdx) -> Result<(), String> {
        self.visit_node(root)
    }

    fn visit_node(&mut self, idx: NodeIdx) -> Result<(), String> {
        let node = &self.nodes[idx.0 as usize];
        match node.tag {
            NodeTag::Program => {
                let first = node.payload[0];
                let len = node.payload[1];
                for i in 0..len {
                    self.visit_node(NodeIdx(first + i))?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

pub fn check(nodes: Vec<Node>, strings: StringTable, root: NodeIdx) -> Result<(), String> {
    let mut sem = Semantic::new(nodes, strings);
    sem.check(root)
}
