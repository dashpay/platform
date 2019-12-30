import {Platform} from "../../Platform";

declare interface createOpts {
    [name:string]: any;
}

export function create(this: Platform, typeLocator: string, opts: createOpts): any {
    const appNames = Object.keys(this.apps);

    //We can either provide of type `dashpay.profile` or if only one schema provided, of type `profile`.
    const [appName, fieldType] = (typeLocator.includes('.')) ? typeLocator.split('.') : [appNames[0], typeLocator];
    this.dpp.setContract(this.apps[appName].schema);

    const document = this.dpp.document.create(fieldType, opts);
    return document;
}

export default create;
