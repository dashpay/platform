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
