import broadcastStateTransition from '../../broadcastStateTransition';
import { Platform } from '../../Platform';
import { IStateTransitionResult } from '../../IStateTransitionResult';

/**
 * Broadcast an unshield transition (shielded pool -> platform address).
 *
 * The Orchard bundle and full state transition must be built externally
 * (e.g., via rs-sdk's `unshield_funds()` or a native wallet library)
 * and serialized to platform binary format.
 *
 * The platform sighash binds `outputAddress || amount` to prevent
 * an attacker from substituting a different destination or amount.
 *
 * @param serializedTransition - Platform-serialized UnshieldTransition bytes
 * @returns Broadcast result
 */
export async function unshield(
  this: Platform,
  serializedTransition: Uint8Array,
): Promise<IStateTransitionResult> {
  this.logger.debug('[Shielded#unshield] Broadcasting unshield transition');
  await this.initialize();

  const { dpp } = this;

  const transition = dpp.stateTransition.createFromBuffer(
    serializedTransition,
    {},
  );

  const result = await broadcastStateTransition(this, await transition, {
    skipValidation: true,
  });

  this.logger.silly('[Shielded#unshield] Broadcasted UnshieldTransition');

  return result;
}

export default unshield;
