import broadcastStateTransition from '../../broadcastStateTransition';
import { Platform } from '../../Platform';
import { IStateTransitionResult } from '../../IStateTransitionResult';

/**
 * Broadcast a shield-from-asset-lock transition (core asset lock -> shielded pool).
 *
 * The Orchard bundle, asset lock proof, ECDSA signature, and full state
 * transition must be built externally (e.g., via rs-sdk's
 * `shield_from_asset_lock()` or a native wallet library) and serialized
 * to platform binary format.
 *
 * @param serializedTransition - Platform-serialized ShieldFromAssetLockTransition bytes
 * @returns Broadcast result
 */
export async function shieldFromAssetLock(
  this: Platform,
  serializedTransition: Uint8Array,
): Promise<IStateTransitionResult> {
  this.logger.debug('[Shielded#shieldFromAssetLock] Broadcasting shield from asset lock');
  await this.initialize();

  const { dpp } = this;

  const transition = dpp.stateTransition.createFromBuffer(
    serializedTransition,
    {},
  );

  const result = await broadcastStateTransition(this, await transition, {
    skipValidation: true,
  });

  this.logger.silly('[Shielded#shieldFromAssetLock] Broadcasted ShieldFromAssetLockTransition');

  return result;
}

export default shieldFromAssetLock;
