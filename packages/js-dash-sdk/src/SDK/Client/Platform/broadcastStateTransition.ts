import crypto from "crypto";
import { Platform } from "./Platform";
import { StateTransitionBroadcastError } from "../../../errors/StateTransitionBroadcastError";
import { IStateTransitionResult } from "./IStateTransitionResult";
import { IPlatformStateProof } from "./IPlatformStateProof";

/**
 * @param {Platform} platform
 * @param stateTransition
 */
export default async function broadcastStateTransition(platform: Platform, stateTransition: any): Promise<IPlatformStateProof|void> {
    const { client, dpp } = platform;

    const result = await dpp.stateTransition.validateStructure(stateTransition);

    if (!result.isValid()) {
        throw new Error(`StateTransition is invalid - ${JSON.stringify(result.getErrors())}`);
    }

    // Subscribing to future result
    const hash = crypto.createHash('sha256')
      .update(stateTransition.toBuffer())
      .digest();

    const stateTransitionResultPromise: IStateTransitionResult = client.getDAPIClient().platform.waitForStateTransitionResult(hash, { prove: true });

    // Broadcasting state transition
    try {
        await client.getDAPIClient().platform.broadcastStateTransition(stateTransition.toBuffer());
    } catch (e) {
        let data;
        let message;

        if (e.data) {
            data = e.data;
        } else if (e.metadata) {
            const errors = e.metadata.get('errors');
            data = {};
            data.errors = errors && errors.length > 0 ? JSON.parse(errors) : errors;
        }

        if (e.details) {
            message = e.details;
        } else {
            message = e.message;
        }

        throw new StateTransitionBroadcastError(e.code, message, data);
    }

    // Waiting for result to return
    const stateTransitionResult = await stateTransitionResultPromise;

    let { error } = stateTransitionResult;

    if (error) {
        throw new StateTransitionBroadcastError(error.code, error.message, error.data);
    }

    return stateTransitionResult.proof;
}
