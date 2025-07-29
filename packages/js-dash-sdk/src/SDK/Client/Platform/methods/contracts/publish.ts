import { Identity, DataContract, DataContractCreateTransition } from '@dashevo/wasm-dpp';
import { Platform } from '../../Platform';
import broadcastStateTransition from '../../broadcastStateTransition';
import { signStateTransition } from '../../signStateTransition';

/**
 * Publish contract onto the platform
 *
 * @param {Platform} this - bound instance class
 * @param dataContract - contract
 * @param identity - identity
 * @return {DataContractCreateTransition}
 */
export default async function publish(
  this: Platform,
  dataContract: DataContract,
  identity: Identity,
): Promise<DataContractCreateTransition> {
  this.logger.debug(`[Contracts#publish] publish data contract ${dataContract.getId()}`);
  await this.initialize();

  // If wasm-sdk is available, delegate to it
  if (this.wasmSdk && this.getAdapter()) {
    const adapter = this.getAdapter()!;
    
    // Get identity private key for signing
    const account = await this.client.getWalletAccount();
    
    // Get the key for data contract operations (index 2)
    const { privateKey: contractPrivateKey } = account.identities
      .getIdentityHDKeyById(identity.getId().toString(), 2);
    
    // Convert private key to WIF format
    const privateKeyWIF = adapter.convertPrivateKeyToWIF(contractPrivateKey);
    
    // Convert identity to hex format
    const identityHex = identity.toBuffer().toString('hex');
    
    // Convert data contract to JSON
    const dataContractJson = JSON.stringify(dataContract.toJSON());
    
    this.logger.debug(`[Contracts#publish] Calling wasm-sdk dataContractCreate`);
    
    // Call wasm-sdk dataContractCreate
    const result = await this.wasmSdk.dataContractCreate(
      dataContractJson,
      identityHex,
      privateKeyWIF
    );
    
    // Acknowledge identifier to handle retry attempts
    this.fetcher.acknowledgeIdentifier(dataContract.getId());
    
    this.logger.debug(`[Contracts#publish] Published data contract ${dataContract.getId()} via wasm-sdk`);
    
    // Return the result as a DataContractCreateTransition
    return result as DataContractCreateTransition;
  }

  const { dpp } = this;

  const dataContractCreateTransition = dpp.dataContract
    .createDataContractCreateTransition(dataContract);

  this.logger.silly(`[Contracts#publish] created data contract create transition ${dataContract.getId()}`);

  await signStateTransition(this, dataContractCreateTransition, identity, 2);
  await broadcastStateTransition(this, dataContractCreateTransition);

  // Acknowledge identifier to handle retry attempts to mitigate
  // state transition propagation lag
  this.fetcher.acknowledgeIdentifier(dataContract.getId());

  this.logger.debug(`[Contracts#publish] publish data contract ${dataContract.getId()}`);

  return dataContractCreateTransition;
}
