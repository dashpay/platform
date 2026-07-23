export class PlatformProofVerificationUnavailableError extends Error {
  constructor(operation: string) {
    super(
      `${operation} requires an authenticated Platform proof verifier; `
      + 'configure platformProofVerifier or use the proof-verifying Wasm SDK',
    );

    Object.setPrototypeOf(this, PlatformProofVerificationUnavailableError.prototype);
  }
}

export default PlatformProofVerificationUnavailableError;
