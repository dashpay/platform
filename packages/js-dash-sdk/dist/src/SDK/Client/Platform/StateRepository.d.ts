/// <reference types="node" />
import DataContract from '@dashevo/dpp/lib/dataContract/DataContract';
import Identity from '@dashevo/dpp/lib/identity/Identity';
import Identifier from '@dashevo/dpp/lib/Identifier';
import Client from '../Client';
declare class StateRepository {
    private readonly client;
    constructor(client: Client);
    fetchIdentity(id: Identifier | string): Promise<Identity | null>;
    fetchDataContract(identifier: Identifier | string): Promise<DataContract | null>;
    isAssetLockTransactionOutPointAlreadyUsed(): Promise<boolean>;
    verifyInstantLock(): Promise<boolean>;
    fetchTransaction(id: string): Promise<{
        data: Buffer;
        height: number;
    }>;
    fetchLatestPlatformCoreChainLockedHeight(): Promise<number>;
}
export default StateRepository;
