export class DashSDKError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'DashSDKError';
  }
}

export class InitializationError extends DashSDKError {
  constructor(message: string) {
    super(message);
    this.name = 'InitializationError';
  }
}

export class NetworkError extends DashSDKError {
  constructor(message: string) {
    super(message);
    this.name = 'NetworkError';
  }
}

export class ValidationError extends DashSDKError {
  constructor(message: string) {
    super(message);
    this.name = 'ValidationError';
  }
}

export class StateTransitionError extends DashSDKError {
  constructor(message: string, public code?: number) {
    super(message);
    this.name = 'StateTransitionError';
  }
}

export class NotFoundError extends DashSDKError {
  constructor(resource: string, id: string) {
    super(`${resource} with ID ${id} not found`);
    this.name = 'NotFoundError';
  }
}

export class InsufficientBalanceError extends DashSDKError {
  constructor(required: number, available: number) {
    super(`Insufficient balance. Required: ${required}, available: ${available}`);
    this.name = 'InsufficientBalanceError';
  }
}

export class TimeoutError extends DashSDKError {
  constructor(operation: string, timeout: number) {
    super(`${operation} timed out after ${timeout}ms`);
    this.name = 'TimeoutError';
  }
}