import { Identity } from '@dashevo/wasm-dpp';
import { Address, Script } from '@dashevo/dashcore-lib';
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
const MINIMAL_WITHDRAWAL_AMOUNT = 1000000;

type WithdrawalOptions = {
  // TODO: should we leave it? Core fee expected to be a fibonacci number
  coreFeePerByte?: number,
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
    signingKeyIndex: 2,
  },
): Promise<any> {
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

  const { minRelay: minRelayFee } = this.client.wallet.storage
    .getDefaultChainStore().state.fees;
  if (options.coreFeePerByte && options.coreFeePerByte < minRelayFee) {
    throw new Error(`Provided core fee per byte "${options.coreFeePerByte}" is less than minimal network relay fee "${minRelayFee}"`);
  }
  const relayFee = options.coreFeePerByte || minRelayFee;
  const coreFeePerByte = nearestGreaterFibonacci(relayFee);

  const revision = identity.getRevision();

  const identityCreditWithdrawalTransition = dpp.identity
    .createIdentityCreditWithdrawalTransition(
      identity.getId(),
      BigInt(amount),
      coreFeePerByte,
      DEFAULT_POOLING,
      // @ts-ignore
      outputScript.toBuffer(),
      BigInt(revision + 1),
    );

  this.logger.silly('[Identity#creditWithdrawal] Created IdentityCreditWithdrawalTransition');

  await signStateTransition(
    this,
    identityCreditWithdrawalTransition,
    identity,
    options.signingKeyIndex,
  );

  await broadcastStateTransition(this, identityCreditWithdrawalTransition, {
    skipValidation: true,
  });

  this.logger.silly('[Identity#creditWithdrawal] Broadcasted IdentityCreditWithdrawalTransition');

  return true;
}

export default creditWithdrawal;
