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

    const result = await dpp.stateTransition.validateBasic(stateTransition);

    if (!result.isValid()) {
        throw new Error(`StateTransition is invalid - ${JSON.stringify(result.getErrors())}`);
    }

    // Subscribing to future result
    const hash = crypto.createHash('sha256')
      .update(stateTransition.toBuffer())
      .digest();

    const serializedStateTransition = stateTransition.toBuffer();

    try {
        await client.getDAPIClient().platform.broadcastStateTransition(serializedStateTransition);
    } catch (e) {
        let data;
        let message;

        if (e.data) {
            data = e.data;
        } else if (e.metadata) {
            // Due to an unknown bug in the minifier, `get` method of the metadata can be stripped off.
            // See the comment in the 'else' branch for more details
            if (typeof e.metadata.get === 'function') {
                const errors = e.metadata.get('errors');
                data = {};
                data.errors = errors && errors.length > 0 ? JSON.parse(errors) : errors;
            } else {
                // This code can be executed only if deserialization failed and no errors
                // were provided in the metadata, so we can deserialize here again
                // and see the details locally
                try {
                    await dpp.stateTransition.createFromBuffer(serializedStateTransition);
                } catch (deserializationError) {
                    data = {};
                    data.errors = deserializationError.errors;
                    data.rawStateTransition = deserializationError.rawStateTransition;
                }
            }
        }

        if (e.details) {
            message = e.details;
        } else {
            message = e.message;
        }

        throw new StateTransitionBroadcastError(e.code, message, data);
    }

    // Waiting for result to return
    const stateTransitionResult: IStateTransitionResult = await client.getDAPIClient().platform.waitForStateTransitionResult(hash, { prove: true });

    let { error } = stateTransitionResult;

    if (error) {
        throw new StateTransitionBroadcastError(error.code, error.message, error.data);
    }

    return stateTransitionResult.proof;
}
