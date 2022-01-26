import { Platform } from "../../Platform";
import broadcastStateTransition from "../../broadcastStateTransition";
import { signStateTransition } from "../../signStateTransition";

/**
 * Publish contract onto the platform
 *
 * @param {Platform} this - bound instance class
 * @param {DataContract} dataContract - contract
 * @param identity - identity
 * @return {DataContractUpdateTransition}
 */
export default async function update(this: Platform, dataContract: any, identity: any): Promise<any> {
  await this.initialize();

  const { dpp } = this;

  // Clone contract
  const updatedDataContract = await this.dpp.dataContract.createFromObject(
    dataContract.toObject(),
  );

  updatedDataContract.incrementVersion();

  const dataContractUpdateTransition = dpp.dataContract.createDataContractUpdateTransition(updatedDataContract);

  await signStateTransition(this, dataContractUpdateTransition, identity);
  await broadcastStateTransition(this, dataContractUpdateTransition);

  // Update app with updated data contract if available
  for (const appName of this.client.getApps().getNames()) {
    const appDefinition = this.client.getApps().get(appName);
    if (appDefinition.contractId.equals(updatedDataContract.getId()) && appDefinition.contract) {
      appDefinition.contract = updatedDataContract;
    }
  }

  return dataContractUpdateTransition;
}
