import { Identity } from '@dashevo/wasm-dpp';
import { Address, Script } from '@dashevo/dashcore-lib';
import { Metadata } from '@dashevo/dapi-client/lib/methods/platform/response/Metadata';
import broadcastStateTransition from '../../broadcastStateTransition';
import { Platform } from '../../Platform';
import { signStateTransition } from '../../signStateTransition';
import { nearestGreaterFibonacci } from '../../../../../utils/fibonacci';

export const STATUSES = {
  PENDING: 0,
  POOLED: 1,
  BROADCASTED: 2,
  COMPLETED: 3,
};

// Implement remaining pooling types when they ready on drive side
const DEFAULT_POOLING = 0;

// TODO: add to dashcore-lib
// Asset unlock TX size is fixed with the default pooling
// since it has zero inputs and only one output
const ASSET_UNLOCK_TX_SIZE = 190;

// Minimal accepted core fee per byte to avoid low fee error from core
const MIN_ASSET_UNLOCK_CORE_FEE_PER_BYTE = 1;

// Minimal withdrawal amount in credits to avoid dust error from core
const MINIMAL_WITHDRAWAL_AMOUNT = ASSET_UNLOCK_TX_SIZE * MIN_ASSET_UNLOCK_CORE_FEE_PER_BYTE * 1000;

type WithdrawalOptions = {
  signingKeyIndex: number
};

/** Creates platform credits withdrawal request
 * @param identity - identity to withdraw from
 * @param amount - amount of credits to withdraw
 * @param to - Dash L1 address
 * @param options - withdrawal options
 */
export async function creditWithdrawal(
  this: Platform,
  identity: Identity,
  amount: number,
  to: string,
  options: WithdrawalOptions = {
    signingKeyIndex: 3,
  },
): Promise<Metadata> {
  await this.initialize();

  const { dpp } = this;

  let toAddress: Address;
  try {
    toAddress = new Address(to, this.client.network);
  } catch (e) {
    throw new Error(`Invalid core recipient "${to}" for network ${this.client.network}`);
  }
  this.logger.debug(`[Identity#creditWithdrawal] credits withdrawal from ${identity.getId().toString()} to ${toAddress.toString()} with amount ${amount}`);

  const outputScript = Script.buildPublicKeyHashOut(toAddress);

  const balance = identity.getBalance();
  if (amount > balance) {
    throw new Error(`Withdrawal amount "${amount}" is bigger that identity balance "${balance}"`);
  }

  if (amount < MINIMAL_WITHDRAWAL_AMOUNT) {
    throw new Error(`Withdrawal amount "${amount}" is less than minimal allowed withdrawal amount "${MINIMAL_WITHDRAWAL_AMOUNT}"`);
  }

  if (!this.client.wallet) {
    throw new Error('Wallet is not initialized');
  }

  // Divide by 1000 as stated in policy for GetDustThreshold
  // https://github.com/dashpay/dash/blob/master/src/policy/policy.cpp#L23
  const minRelayFeePerByte = Math.ceil(this.client.wallet.storage
    .getDefaultChainStore().state.fees.minRelay / 1000);

  const coreFeePerByte = nearestGreaterFibonacci(minRelayFeePerByte);

  const identityNonce = await this.nonceManager.bumpIdentityNonce(identity.getId());

  const identityCreditWithdrawalTransition = dpp.identity
    .createIdentityCreditWithdrawalTransition(
      identity.getId(),
      BigInt(amount),
      coreFeePerByte,
      DEFAULT_POOLING,
      // @ts-ignore
      outputScript.toBuffer(),
      BigInt(identityNonce),
    );

  this.logger.silly('[Identity#creditWithdrawal] Created IdentityCreditWithdrawalTransition');

  await signStateTransition(
    this,
    identityCreditWithdrawalTransition,
    identity,
    options.signingKeyIndex,
  );

  // Skipping validation because it's already done above
  const stateTransitionResult = await broadcastStateTransition(
    this,
    identityCreditWithdrawalTransition,
    {
      skipValidation: true,
    },
  );

  this.logger.silly('[Identity#creditWithdrawal] Broadcasted IdentityCreditWithdrawalTransition');

  return stateTransitionResult.metadata;
}

export default creditWithdrawal;
