import {Platform} from "../../Platform";

declare type ContractIdentifier = string;

export function fetch(this: Platform, identifier: ContractIdentifier): any {
    // TODO : if identifier is contractId then fetch directly
    // if not, fetch from dpns the contract id and fetch it
    throw new Error('Implementation missing in dependencies.');


    // const contract = await this.client.fetchContract(identifier);
    const contract = null;
    return contract;
}

export default fetch;
