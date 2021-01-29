import { IPlatformStateProof } from "./IPlatformStateProof";

export interface IStateTransitionResult {
    proof?: IPlatformStateProof,
    error?: {
        code: number,
        message: string,
        data: any,
    }
}
