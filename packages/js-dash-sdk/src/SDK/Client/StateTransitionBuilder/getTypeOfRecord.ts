// @ts-ignore
import Document from "@dashevo/dpp/lib/document/Document";
// @ts-ignore
import DataContract from "@dashevo/dpp/lib/dataContract/DataContract";
// @ts-ignore
import Identity from "@dashevo/dpp/lib/identity/Identity";
import {Record, StateTransitionBuilderTypes} from "./StateTransitionBuilder";

const getTypeOfRecord = (record: Record) => {
    if(!record) return null;
    switch (record.constructor.prototype) {
        case Document.prototype:
            return StateTransitionBuilderTypes.DOCUMENT;
        case DataContract.prototype:
            return StateTransitionBuilderTypes.CONTRACT;
        case Identity.prototype:
            return StateTransitionBuilderTypes.IDENTITY;
        default:
            throw new Error('Unhandled record type');
    }
}
export default getTypeOfRecord;
