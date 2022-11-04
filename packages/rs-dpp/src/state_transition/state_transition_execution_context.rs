use super::fees::operations::Operation;

#[derive(Debug, Clone, Default)]

pub struct StateTransitionExecutionContext {
    operations: Vec<Operation>,
}

impl StateTransitionExecutionContext {
    pub fn add_operation(&mut self, operation: Operation) {
        self.operations.push(operation);
    }

    pub fn set_operations(&mut self, operations: Vec<Operation>) {
        self.operations = operations
    }

    pub fn get_operations(&self) -> &[Operation] {
        &self.operations
    }
}
