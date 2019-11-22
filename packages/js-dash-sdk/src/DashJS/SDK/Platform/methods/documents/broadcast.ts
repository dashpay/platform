import {Platform} from "../../Platform";


export function broadcast(this: Platform, document: any): any {
    document.removeMetadata();
    const result = this.dpp.document.validate(document);
    if (!result.isValid()) {
        throw new Error('Invalid request');
    }
    throw new Error('Implementation missing in dependencies.');

    // const {
    //     serializedTransaction,
    //     serializedPacket,
    // } = this.prepareStateTransition(document, this.sender.buser, this.sender.buser.privateKey);
    //
    //
    // const txid = await this.sender.broadcastTransition(
    //     serializedTransaction,
    //     serializedPacket,
    // );
    //
    // return txid;
}

export default broadcast;
