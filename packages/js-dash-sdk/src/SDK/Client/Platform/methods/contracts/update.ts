import { Platform } from '../../Platform';
import broadcastStateTransition from '../../broadcastStateTransition';
import { signStateTransition } from '../../signStateTransition';

/**
 * Publish contract onto the platform
 *
 * @param {Platform} this - bound instance class
 * @param {DataContract} dataContract - contract
 * @param identity - identity
 * @return {DataContractUpdateTransition}
 */
export default async function update(
  this: Platform,
  dataContract: any,
  identity: any,
): Promise<any> {
  this.logger.debug(`[DataContract#update] Update data contract ${dataContract.getId()}`);
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
    
    // Clone and increment version
    const updatedDataContract = dataContract.clone();
    updatedDataContract.incrementVersion();
    
    // Convert updated data contract to JSON
    const dataContractJson = JSON.stringify(updatedDataContract.toJSON());
    
    this.logger.debug(`[DataContract#update] Calling wasm-sdk dataContractUpdate`);
    
    // Call wasm-sdk dataContractUpdate
    const result = await this.wasmSdk.dataContractUpdate(
      dataContractJson,
      identityHex,
      privateKeyWIF
    );
    
    // Update app with updated data contract if available
    // eslint-disable-next-line
    for (const appName of this.client.getApps().getNames()) {
      const appDefinition = this.client.getApps().get(appName);
      if (appDefinition.contractId.equals(updatedDataContract.getId()) && appDefinition.contract) {
        appDefinition.contract = updatedDataContract;
      }
    }
    
    this.logger.debug(`[DataContract#update] Updated data contract ${dataContract.getId()} via wasm-sdk`);
    
    return result;
  }

  const { dpp } = this;

  // Clone contract
  const updatedDataContract = dataContract.clone();

  updatedDataContract.incrementVersion();

  const identityId = identity.getId();
  const dataContractId = dataContract.getId();

  const identityContractNonce = await this.nonceManager
    .bumpIdentityContractNonce(identityId, dataContractId);

  const dataContractUpdateTransition = dpp.dataContract
    .createDataContractUpdateTransition(updatedDataContract, identityContractNonce);

  this.logger.silly(`[DataContract#update] Created data contract update transition ${dataContract.getId()}`);

  await signStateTransition(this, dataContractUpdateTransition, identity, 2);
  // Broadcast state transition also wait for the result to be obtained
  await broadcastStateTransition(this, dataContractUpdateTransition);

  this.logger.silly(`[DataContract#update] Broadcasted data contract update transition ${dataContract.getId()}`);
  // Update app with updated data contract if available
  // eslint-disable-next-line
  for (const appName of this.client.getApps().getNames()) {
    const appDefinition = this.client.getApps().get(appName);
    if (appDefinition.contractId.equals(updatedDataContract.getId()) && appDefinition.contract) {
      appDefinition.contract = updatedDataContract;
    }
  }

  this.logger.debug(`[DataContract#updated] Update data contract ${dataContract.getId()}`);
  return dataContractUpdateTransition;
}
