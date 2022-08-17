import crypto from "crypto";
import { Platform } from "./Platform";
import { StateTransitionBroadcastError } from "../../../errors/StateTransitionBroadcastError";
import { IStateTransitionResult } from "./IStateTransitionResult";

const ResponseError = require('@dashevo/dapi-client/lib/transport/errors/response/ResponseError');
const InvalidRequestDPPError = require('@dashevo/dapi-client/lib/transport/errors/response/InvalidRequestDPPError');

const createGrpcTransportError = require('@dashevo/dapi-client/lib/transport/GrpcTransport/createGrpcTransportError');

const GrpcError = require('@dashevo/grpc-common/lib/server/error/GrpcError');

/**
 * @param {Platform} platform
 * @param {Object} [options]
 * @param {boolean} [options.skipValidation=false]
 *
 * @param stateTransition
 */
export default async function broadcastStateTransition(
  platform: Platform,
  stateTransition: any,
  options: { skipValidation?: boolean; } = {},
): Promise<IStateTransitionResult|void> {
    const { client, dpp } = platform;

    if (!options.skipValidation) {
      const result = await dpp.stateTransition.validateBasic(stateTransition);

      if (!result.isValid()) {
        const consensusError = result.getFirstError();

        throw new StateTransitionBroadcastError(
          consensusError.getCode(),
          consensusError.message,
          consensusError,
        );
      }
    }

    // Subscribing to future result
    const hash = crypto.createHash('sha256')
      .update(stateTransition.toBuffer())
      .digest();

    const serializedStateTransition = stateTransition.toBuffer();

    try {
        await client.getDAPIClient().platform.broadcastStateTransition(serializedStateTransition);
    } catch (error) {
        if (error instanceof ResponseError) {
            let cause = error;

            // Pass DPP consensus error directly to avoid
            // additional wrappers
            if (cause instanceof InvalidRequestDPPError) {
                cause = cause.getConsensusError();
            }

            throw new StateTransitionBroadcastError(
                cause.getCode(),
                cause.message,
                cause,
            );
        }

        throw error;
    }

    // Waiting for result to return
    const stateTransitionResult: IStateTransitionResult = await client.getDAPIClient().platform.waitForStateTransitionResult(hash, { prove: true });

    let { error } = stateTransitionResult;

    if (error) {
        // Create DAPI response error from gRPC error passed as gRPC response
        const grpcError = new GrpcError(error.code, error.message, error.data);

        let cause = createGrpcTransportError(grpcError);

        // Pass DPP consensus error directly to avoid
        // additional wrappers
        if (cause instanceof InvalidRequestDPPError) {
            cause = cause.getConsensusError();
        }

        throw new StateTransitionBroadcastError(
            cause.getCode(),
            cause.message,
            cause,
        );
    }

    return stateTransitionResult;
}
