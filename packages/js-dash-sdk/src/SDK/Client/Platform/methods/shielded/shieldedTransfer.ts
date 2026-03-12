import broadcastStateTransition from '../../broadcastStateTransition';
import { Platform } from '../../Platform';
import { IStateTransitionResult } from '../../IStateTransitionResult';

/**
 * Broadcast a shielded transfer (shielded-to-shielded) transition.
 *
 * The Orchard bundle and full state transition must be built externally
 * (e.g., via rs-sdk's `transfer_shielded()` or a native wallet library)
 * and serialized to platform binary format before passing here.
 *
 * Authentication is provided entirely by Orchard spend authorization
 * signatures within the bundle — no identity signing needed.
 *
 * @param serializedTransition - Platform-serialized ShieldedTransferTransition bytes
 * @returns Broadcast result
 */
export async function shieldedTransfer(
  this: Platform,
  serializedTransition: Uint8Array,
): Promise<IStateTransitionResult> {
  this.logger.debug('[Shielded#shieldedTransfer] Broadcasting shielded transfer');
  await this.initialize();

  const { dpp } = this;

  const transition = dpp.stateTransition.createFromBuffer(
    serializedTransition,
    {},
  );

  const result = await broadcastStateTransition(this, await transition, {
    skipValidation: true,
  });

  this.logger.silly('[Shielded#shieldedTransfer] Broadcasted ShieldedTransferTransition');

  return result;
}

export default shieldedTransfer;
