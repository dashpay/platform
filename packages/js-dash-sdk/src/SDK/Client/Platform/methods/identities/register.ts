import { Platform } from '../../Platform';
import broadcastStateTransition from '../../broadcastStateTransition';

/**
 * Register identities to the platform
 *
 * @param {number} [fundingAmount=1000000] - funding amount in duffs
 * @returns {Identity} identity - a register and funded identity
 */
export default async function register(
  this: Platform,
  fundingAmount : number = 1000000,
): Promise<any> {
  this.logger.debug(`[Identity#register] Register identity with funding amount ${fundingAmount}`);
  await this.initialize();

  const { client } = this;
  const account = await client.getWalletAccount();

  // Create asset lock transaction
  const {
    transaction: assetLockTransaction,
    privateKey: assetLockPrivateKey,
    outputIndex: assetLockOutputIndex,
  } = await this.identities.utils.createAssetLockTransaction(fundingAmount);

  // Broadcast Asset Lock transaction
  await account.broadcastTransaction(assetLockTransaction);
  this.logger.silly(`[Identity#register] Broadcasted asset lock transaction "${assetLockTransaction.hash}"`);

  // Create asset lock proof
  const assetLockProof = await this.identities.utils
    .createAssetLockProof(assetLockTransaction, assetLockOutputIndex);
  this.logger.silly(`[Identity#register] Created asset lock proof with tx "${assetLockTransaction.hash}"`);

  // If wasm-sdk is available, delegate to it
  if (this.wasmSdk && this.getAdapter()) {
    const adapter = this.getAdapter()!;
    
    // Get identity index for HD key derivation
    const identityIndex = await account.getUnusedIdentityIndex();
    
    // Generate public keys for the identity
    const publicKeys: any[] = [];
    
    // Authentication master key (id: 0)
    const { privateKey: identityMasterPrivateKey } = account.identities
      .getIdentityHDKeyByIndex(identityIndex, 0);
    const identityMasterPublicKey = identityMasterPrivateKey.toPublicKey();
    publicKeys.push({
      id: 0,
      type: 0, // ECDSA_SECP256K1
      purpose: 0, // AUTHENTICATION
      securityLevel: 0, // MASTER
      data: identityMasterPublicKey.toBuffer().toString('base64'),
      readOnly: false,
    });
    
    // Authentication high level key (id: 1)
    const { privateKey: identityHighAuthPrivateKey } = account.identities
      .getIdentityHDKeyByIndex(identityIndex, 1);
    const identityHighAuthPublicKey = identityHighAuthPrivateKey.toPublicKey();
    publicKeys.push({
      id: 1,
      type: 0, // ECDSA_SECP256K1
      purpose: 0, // AUTHENTICATION
      securityLevel: 2, // HIGH
      data: identityHighAuthPublicKey.toBuffer().toString('base64'),
      readOnly: false,
    });
    
    // Authentication critical level key (id: 2)
    const { privateKey: identityCriticalAuthPrivateKey } = account.identities
      .getIdentityHDKeyByIndex(identityIndex, 2);
    const identityCriticalAuthPublicKey = identityCriticalAuthPrivateKey.toPublicKey();
    publicKeys.push({
      id: 2,
      type: 0, // ECDSA_SECP256K1
      purpose: 0, // AUTHENTICATION
      securityLevel: 1, // CRITICAL
      data: identityCriticalAuthPublicKey.toBuffer().toString('base64'),
      readOnly: false,
    });
    
    // Transfer key (id: 3)
    const { privateKey: identityTransferPrivateKey } = account.identities
      .getIdentityHDKeyByIndex(identityIndex, 3);
    const identityTransferPublicKey = identityTransferPrivateKey.toPublicKey();
    publicKeys.push({
      id: 3,
      type: 0, // ECDSA_SECP256K1
      purpose: 3, // TRANSFER (3)
      securityLevel: 1, // CRITICAL
      data: identityTransferPublicKey.toBuffer().toString('base64'),
      readOnly: false,
    });
    
    // Convert asset lock proof to hex format for wasm-sdk
    const assetLockProofHex = adapter.convertAssetLockProof(assetLockProof);
    
    // Convert private key to WIF format
    const assetLockPrivateKeyWIF = adapter.convertPrivateKeyToWIF(assetLockPrivateKey);
    
    // Call wasm-sdk identityCreate
    const result = await this.wasmSdk.identityCreate(
      assetLockProofHex,
      assetLockPrivateKeyWIF,
      JSON.stringify(publicKeys)
    );
    
    // Parse the identity ID from the result
    const identityId = result.identity?.id || result.id;
    
    // Store identity in wallet
    account.storage
      .getWalletStore(account.walletId)
      .insertIdentityIdAtIndex(
        identityId.toString(),
        identityIndex,
      );
    
    // Acknowledge identifier to handle retry attempts
    this.fetcher.acknowledgeIdentifier(identityId);
    
    // Fetch the created identity
    const registeredIdentity = await this.identities.get(identityId);
    
    if (registeredIdentity === null) {
      throw new Error(`Can't fetch created identity with id ${identityId}`);
    }
    
    this.logger.debug(`[Identity#register] Registered identity "${identityId}"`);
    
    return registeredIdentity;
  }

  // Legacy implementation - will be removed once migration is complete
  const { identity, identityCreateTransition, identityIndex } = await this.identities.utils
    .createIdentityCreateTransition(assetLockProof, assetLockPrivateKey);

  this.logger.silly(`[Identity#register] Created IdentityCreateTransition with identity id "${identity.getId()}" using asset lock tx "${assetLockTransaction.hash}" `);

  // Skipping validation because it's already done in createIdentityCreateTransition
  await broadcastStateTransition(this, identityCreateTransition, {
    skipValidation: true,
  });
  this.logger.silly('[Identity#register] Broadcasted IdentityCreateTransition');

  const identityId = identity.getId();

  // If state transition was broadcast without any errors, import identity to the account
  account.storage
    .getWalletStore(account.walletId)
    .insertIdentityIdAtIndex(
      identityId.toString(),
      identityIndex,
    );

  // Acknowledge identifier to handle retry attempts to mitigate
  // state transition propagation lag
  this.fetcher.acknowledgeIdentifier(identityId);

  const registeredIdentity = await this.identities.get(identityId);

  if (registeredIdentity === null) {
    throw new Error(`Can't fetch created identity with id ${identityId}`);
  }

  // We cannot just return registeredIdentity as we want to
  // keep additional information (assetLockProof and transaction) instance
  identity.setMetadata(registeredIdentity.getMetadata());
  identity.setBalance(registeredIdentity.getBalance());
  identity.setPublicKeys(registeredIdentity.getPublicKeys());

  this.logger.debug(`[Identity#register] Registered identity "${identityId}"`);

  return identity;
}