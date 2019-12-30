import {Platform} from "../../Platform";

declare interface fetchOpts {
    where: any;
    orderBy: any;
    limit: number;
    startAt: number;
    startAfter: number;
}

export function fetch(this: Platform, typeLocator: string, opts: fetchOpts): any {
    const appNames = Object.keys(this.apps);

    //We can either provide of type `dashpay.profile` or if only one schema provided, of type `profile`.
    const [appName, fieldType] = (typeLocator.includes('.')) ? typeLocator.split('.') : [appNames[0], typeLocator];
    // FIXME: we may later want a hashmap of schemas and contract IDs

    if(!this.apps[appName]){
        throw new Error(`No app named ${appName} specified.`)
    }
    const app = this.apps[appName];
    if (!app.schema) {
        throw new Error(`Missing schema for ${appName}`)
    }
    const schema = app.schema;
    if (!app.contractId) {
        throw new Error(`Missing contract ID for ${appName}`)
    }
    const contractId = app.contractId;

    // FIXME : In JS-DPP 0.10, change altered the way we actually manipulate contract id.
    // Therefore, below code is now invalid

    // console.dir(this.dpp.dataContract.factory.validateDataContract(schema),{depth:null});
    // this.dpp.dataContract.setContract(this.schemas[contractName]);
    // this.dpp.dataContract.setDocumentSchema(this.schemas[contractName]);
    // const contractId = this.dpp.getContract().getId();
    // return this.client.fetchDocuments(contractId, fieldType, opts);
}

export default fetch;
