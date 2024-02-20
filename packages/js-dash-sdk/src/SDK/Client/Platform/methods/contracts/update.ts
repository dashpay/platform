import GrpcErrorCodes from '@dashevo/grpc-common/lib/server/error/GrpcErrorCodes';
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

  const { dpp } = this;

  // Clone contract
  const updatedDataContract = dataContract.clone();

  updatedDataContract.incrementVersion();

  const identityId = identity.getId();
  const dataContractId = dataContract.getId();

  const identityContractNonce = await this.nonceManager
    .getIdentityContractNonce(identityId, dataContractId) + 1;

  const dataContractUpdateTransition = dpp.dataContract
    .createDataContractUpdateTransition(updatedDataContract, BigInt(identityContractNonce));

  this.logger.silly(`[DataContract#update] Created data contract update transition ${dataContract.getId()}`);

  await signStateTransition(this, dataContractUpdateTransition, identity, 2);
  try {
    // Broadcast state transition also wait for the result to be obtained
    await broadcastStateTransition(this, dataContractUpdateTransition);
    this.nonceManager
      .setIdentityContractNonce(identityId, dataContractId, identityContractNonce);
  } catch (e) {
    // Deadline exceeded would mean that state transition didn't make it to the block,
    // so we will not update nonce in this case
    if (e.code !== GrpcErrorCodes.DEADLINE_EXCEEDED) {
      this.nonceManager
        .setIdentityContractNonce(identityId, dataContractId, identityContractNonce);
    }

    throw e;
  }

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
