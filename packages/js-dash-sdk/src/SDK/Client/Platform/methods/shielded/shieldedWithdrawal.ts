import broadcastStateTransition from '../../broadcastStateTransition';
import { Platform } from '../../Platform';
import { IStateTransitionResult } from '../../IStateTransitionResult';

/**
 * Broadcast a shielded withdrawal transition (shielded pool -> core L1 address).
 *
 * The Orchard bundle and full state transition must be built externally
 * (e.g., via rs-sdk's `withdraw_shielded()` or a native wallet library)
 * and serialized to platform binary format.
 *
 * The platform sighash binds `outputScript || amount` to prevent
 * an attacker from substituting a different L1 destination or amount.
 *
 * @param serializedTransition - Platform-serialized ShieldedWithdrawalTransition bytes
 * @returns Broadcast result
 */
export async function shieldedWithdrawal(
  this: Platform,
  serializedTransition: Uint8Array,
): Promise<IStateTransitionResult> {
  this.logger.debug('[Shielded#shieldedWithdrawal] Broadcasting shielded withdrawal');
  await this.initialize();

  const { dpp } = this;

  const transition = dpp.stateTransition.createFromBuffer(
    serializedTransition,
    {},
  );

  const result = await broadcastStateTransition(this, await transition, {
    skipValidation: true,
  });

  this.logger.silly('[Shielded#shieldedWithdrawal] Broadcasted ShieldedWithdrawalTransition');

  return result;
}

export default shieldedWithdrawal;
