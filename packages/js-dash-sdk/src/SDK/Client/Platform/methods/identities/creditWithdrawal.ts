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
  signingKeyIndex?: number
  toAddress?: string
};

/** Creates platform credits withdrawal request
 * @param identity - identity to withdraw from
 * @param amount - amount of credits to withdraw
 * @param options - withdrawal options
 * @param [options] - withdrawal options
 * @param [options.toAddress] - withdrawal destination address
 */
export async function creditWithdrawal(
  this: Platform,
  identity: Identity,
  amount: number,
  options: WithdrawalOptions = { },
): Promise<Metadata> {
  await this.initialize();

  // eslint-disable-next-line no-param-reassign
  options = {
    signingKeyIndex: 3,
    ...options,
  };

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

  // If wasm-sdk is available, delegate to it
  if (this.wasmSdk && this.getAdapter()) {
    const adapter = this.getAdapter()!;
    
    // Get the identity's private key for signing
    const account = await this.client.getWalletAccount();
    
    // Get the signing key (default to transfer key at index 3)
    const { privateKey: signingPrivateKey } = account.identities
      .getIdentityHDKeyById(identity.getId().toString(), options.signingKeyIndex!);
    
    // Convert private key to WIF format
    const privateKeyWIF = adapter.convertPrivateKeyToWIF(signingPrivateKey);
    
    // Use provided address or generate from wallet
    let toAddress = options.toAddress;
    if (!toAddress) {
      // Get a new address from wallet for withdrawal
      const { address } = await account.getUnusedAddress();
      toAddress = address.toString();
    } else {
      // Validate the provided address
      try {
        new Address(toAddress, this.client.network);
      } catch (e) {
        throw new Error(`Invalid core recipient "${toAddress}" for network ${this.client.network}`);
      }
    }
    
    // Calculate core fee per byte
    const minRelayFeePerByte = Math.ceil(this.client.wallet.storage
      .getDefaultChainStore().state.fees.minRelay / 1000);
    const coreFeePerByte = nearestGreaterFibonacci(minRelayFeePerByte);
    
    this.logger.debug(`[Identity#creditWithdrawal] credits withdrawal from ${identity.getId().toString()} to ${toAddress} with amount ${amount}`);
    
    // Call wasm-sdk identityCreditWithdrawal
    const result = await this.wasmSdk.identityCreditWithdrawal(
      identity.getId().toString(),
      toAddress,
      amount,
      coreFeePerByte,
      privateKeyWIF,
      options.signingKeyIndex
    );
    
    this.logger.debug(`[Identity#creditWithdrawal] Withdrawal request created`);
    
    // Return metadata from the result
    return result.metadata || new Metadata();
  }

  // Legacy implementation - will be removed once migration is complete
  const { dpp } = this;

  let outputScriptBytes: Buffer | undefined;
  if (options.toAddress) {
    let toAddress: Address;
    try {
      toAddress = new Address(options.toAddress, this.client.network);
    } catch (e) {
      throw new Error(`Invalid core recipient "${options.toAddress}" for network ${this.client.network}`);
    }

    const outputScript = Script.buildPublicKeyHashOut(toAddress);
    // @ts-ignore
    outputScriptBytes = outputScript.toBuffer();

    this.logger.debug(`[Identity#creditWithdrawal] credits withdrawal from ${identity.getId().toString()} to ${toAddress.toString()} with amount ${amount}`);
  } else {
    this.logger.debug(`[Identity#creditWithdrawal] credits withdrawal from ${identity.getId().toString()} to recent withdrawal address with amount ${amount}`);
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
      outputScriptBytes,
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