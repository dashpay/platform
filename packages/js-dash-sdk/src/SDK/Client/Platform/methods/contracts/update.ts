import { Platform } from "../../Platform";
import broadcastStateTransition from "../../broadcastStateTransition";
import { signStateTransition } from "../../signStateTransition";

/**
 * Publish contract onto the platform
 *
 * @param {Platform} this - bound instance class
 * @param dataContract - contract
 * @param identity - identity
 * @return {DataContractUpdateTransition}
 */
export default async function update(this: Platform, dataContract: any, identity: any): Promise<any> {
    await this.initialize();

    const { dpp } = this;

    const dataContractUpdateTransition = dpp.dataContract.createDataContractUpdateTransition(dataContract);

    await signStateTransition(this, dataContractUpdateTransition, identity);
    await broadcastStateTransition(this, dataContractUpdateTransition);

    return dataContractUpdateTransition;
}
