import { IPlatformStateProof } from './IPlatformStateProof';

export type Metadata = {
  height: number,
  coreChainLockedHeight: number,
  timeMs: number,
  protocolVersion: number,
};

export interface IStateTransitionResult {
  metadata: Metadata,
  proof?: IPlatformStateProof,
  error?: {
    code: number,
    message: string,
    data: any,
  }
}
