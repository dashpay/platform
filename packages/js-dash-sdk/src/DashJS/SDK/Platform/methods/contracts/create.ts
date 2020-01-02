import {Platform} from "../../Platform";

export function create(this: Platform, documentDefinitions: any, identity: any): Promise<any> {
    return this.dpp.dataContract.create(identity.getId(), documentDefinitions);;
}

export default create;
