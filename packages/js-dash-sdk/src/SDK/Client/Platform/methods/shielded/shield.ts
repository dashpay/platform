import broadcastStateTransition from '../../broadcastStateTransition';
import { Platform } from '../../Platform';
import { IStateTransitionResult } from '../../IStateTransitionResult';

/**
 * Broadcast a shield transition (transparent platform addresses -> shielded pool).
 *
 * The Orchard bundle, platform address witnesses, and full state transition
 * must be built externally (e.g., via rs-sdk's `shield_funds()` or a native
 * wallet library) and serialized to platform binary format.
 *
 * @param serializedTransition - Platform-serialized ShieldTransition bytes
 * @returns Broadcast result
 */
export async function shield(
  this: Platform,
  serializedTransition: Uint8Array,
): Promise<IStateTransitionResult> {
  this.logger.debug('[Shielded#shield] Broadcasting shield transition');
  await this.initialize();

  const { dpp } = this;

  const transition = dpp.stateTransition.createFromBuffer(
    serializedTransition,
    {},
  );

  const result = await broadcastStateTransition(this, await transition, {
    skipValidation: true,
  });

  this.logger.silly('[Shielded#shield] Broadcasted ShieldTransition');

  return result;
}

export default shield;
