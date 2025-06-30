import { SDK } from '../SDK';
import { getWasmSdk } from './WasmLoader';
import { StateTransitionResult, BroadcastOptions } from './types';
import { StateTransitionError, NetworkError, TimeoutError } from '../utils/errors';

export class StateTransitionBroadcaster {
  constructor(private sdk: SDK) {}

  async broadcast(
    stateTransition: any,
    options: BroadcastOptions = {}
  ): Promise<StateTransitionResult> {
    const wasm = getWasmSdk();
    const wasmSdk = this.sdk.getWasmSdk();
    
    // Get context for the broadcast
    const context = await this.sdk.createContext();
    
    try {
      // Validate the state transition unless skipped
      if (!options.skipValidation) {
        await this.validate(stateTransition);
      }
      
      // Broadcast with retry logic
      const retries = options.retries ?? this.sdk.getOptions().retries ?? 3;
      let lastError: Error | null = null;
      
      for (let attempt = 0; attempt <= retries; attempt++) {
        try {
          const result = await wasm.broadcastStateTransition(
            wasmSdk,
            stateTransition,
            options.skipValidation
          );
          
          // Parse the result
          return {
            stateTransition,
            metadata: {
              height: result.blockHeight,
              coreChainLockedHeight: result.coreChainLockedHeight,
              epoch: result.epoch,
              timeMs: result.timeMs,
              protocolVersion: result.protocolVersion,
              fee: result.fee
            }
          };
        } catch (error: any) {
          lastError = error;
          
          // Don't retry on validation errors
          if (error.message?.includes('validation') || error.message?.includes('invalid')) {
            throw new StateTransitionError(error.message, error.code);
          }
          
          // Wait before retry (exponential backoff)
          if (attempt < retries) {
            await this.sleep(Math.pow(2, attempt) * 1000);
          }
        }
      }
      
      // All retries failed
      throw new NetworkError(
        `Failed to broadcast after ${retries + 1} attempts: ${lastError?.message}`
      );
    } catch (error: any) {
      if (error instanceof StateTransitionError || error instanceof NetworkError) {
        throw error;
      }
      
      throw new StateTransitionError(
        `Broadcast failed: ${error.message}`,
        error.code
      );
    }
  }

  async waitForConfirmation(
    stateTransitionHash: string,
    timeout: number = 60000
  ): Promise<StateTransitionResult> {
    const wasm = getWasmSdk();
    const wasmSdk = this.sdk.getWasmSdk();
    
    const startTime = Date.now();
    const checkInterval = 2000; // 2 seconds
    
    while (Date.now() - startTime < timeout) {
      try {
        // Check if state transition is confirmed
        const result = await wasm.waitForStateTransition(
          wasmSdk,
          stateTransitionHash,
          true // prove
        );
        
        if (result?.metadata) {
          return {
            stateTransition: result.stateTransition,
            metadata: result.metadata
          };
        }
      } catch (error: any) {
        // Ignore not found errors while waiting
        if (!error.message?.includes('not found')) {
          throw error;
        }
      }
      
      await this.sleep(checkInterval);
    }
    
    throw new TimeoutError('State transition confirmation', timeout);
  }

  private async validate(stateTransition: any): Promise<void> {
    const wasm = getWasmSdk();
    const wasmSdk = this.sdk.getWasmSdk();
    
    try {
      await wasm.validateStateTransition(wasmSdk, stateTransition);
    } catch (error: any) {
      throw new StateTransitionError(
        `Validation failed: ${error.message}`,
        error.code || 0
      );
    }
  }

  private sleep(ms: number): Promise<void> {
    return new Promise(resolve => setTimeout(resolve, ms));
  }
}