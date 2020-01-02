import {Platform} from "../../Platform";

declare type ContractIdentifier = string;

export async function get(this: Platform, identifier: ContractIdentifier): Promise<any> {
    let localContract;

    for (let appName in this.apps) {
        const app = this.apps[appName];
        if (app.contractId === identifier) {
            localContract = app;
            break;
        }
    }

    if (localContract && localContract.contract) {
        return localContract.contract;
    } else {
        // @ts-ignore
        const rawContract = await this.client.getDataContract(identifier)
        const contract = this.dpp.dataContract.createFromSerialized(rawContract);
        const app = {contractId: identifier, contract};
        // If we do not have even the identifier in this.apps, we add it with timestamp as key
        if (localContract === undefined) {
            this.apps[Date.now()] = app
        }
        return app.contract;
    }
}

export default get;
