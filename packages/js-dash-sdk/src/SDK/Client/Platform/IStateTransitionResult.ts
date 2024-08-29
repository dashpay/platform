import { Metadata } from '@dashevo/dapi-client/lib/methods/platform/response/Metadata';
import { IPlatformStateProof } from './IPlatformStateProof';

export interface IStateTransitionResult {
  metadata: Metadata,
  proof?: IPlatformStateProof,
  error?: {
    code: number,
    message: string,
    data: any,
  }
}
