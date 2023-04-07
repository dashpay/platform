/// <reference types="node" />
import { Platform } from '../../Platform';
declare let Identifier: any;
declare let Document: any;
/**
 * @param {WhereCondition[]} [where] - where
 * @param {OrderByCondition[]} [orderBy] - order by
 * @param {number} [limit] - limit
 * @param {string|Buffer|Document|Identifier} [startAt] - start value (included)
 * @param {string|Buffer|Document|Identifier} [startAfter] - start value (not included)
 */
declare interface FetchOpts {
    where?: WhereCondition[];
    orderBy?: OrderByCondition[];
    limit?: number;
    startAt?: string | Buffer | typeof Document | typeof Identifier;
    startAfter?: string | Buffer | typeof Document | typeof Identifier;
}
declare type OrderByCondition = [string, 'asc' | 'desc'];
declare type WhereCondition = [string, '<' | '<=' | '==' | '>' | '>=' | 'in' | 'startsWith' | 'elementMatch' | 'length' | 'contains', WhereCondition | any];
/**
 * Get documents from the platform
 *
 * @param {Platform} this bound instance class
 * @param {string} typeLocator type locator
 * @param {FetchOpts} opts - MongoDB style query
 * @returns documents
 */
export declare function get(this: Platform, typeLocator: string, opts: FetchOpts): Promise<any>;
export default get;
