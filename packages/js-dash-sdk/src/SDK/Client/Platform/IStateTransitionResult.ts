import { Metadata } from '@dashevo/dapi-client/lib/methods/platform/response/Metadata.js';
import { IPlatformStateProof } from './IPlatformStateProof.js';

export interface IStateTransitionResult {
  metadata: Metadata,
  proof?: IPlatformStateProof,
  error?: {
    code: number,
    message: string,
    data?: Buffer,
  }
}
