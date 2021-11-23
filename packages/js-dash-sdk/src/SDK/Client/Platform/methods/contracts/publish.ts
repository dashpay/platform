import { Platform } from "../../Platform";
import broadcastStateTransition from "../../broadcastStateTransition";
import { signStateTransition } from "../../signStateTransition";

/**
 * Publish contract onto the platform
 *
 * @param {Platform} this - bound instance class
 * @param dataContract - contract
 * @param identity - identity
 * @return {DataContractCreateTransition}
 */
export default async function publish(this: Platform, dataContract: any, identity: any): Promise<any> {
    await this.initialize();

    const { dpp } = this;

    const dataContractCreateTransition = dpp.dataContract.createDataContractCreateStateTransition(dataContract);

    await signStateTransition(this, dataContractCreateTransition, identity);
    await broadcastStateTransition(this, dataContractCreateTransition);

    return dataContractCreateTransition;
}
