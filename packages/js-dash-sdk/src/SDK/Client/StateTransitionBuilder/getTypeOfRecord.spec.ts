import {expect} from 'chai';
import getTypeOfRecord from "./getTypeOfRecord";
import 'mocha';
import DataContract from "@dashevo/dpp/lib/dataContract/DataContract";

describe('StateTransitionBuilder - getTypeOfRecord', function suite() {
    this.timeout(10000);
    it('should get the correct type', function () {
        const dataContract = new DataContract({
                contractId: 'At44pvrZXLwjbJp415E2kjav49goGosRF3SB1WW1QJoG',
                version: 1,
                schema: 'https://schema.dash.org/dpp-0-4-0/meta/data-contract',
                documents: {note: {text: 'something'}},
                definitions: {}
            }
        );
        const type = getTypeOfRecord(dataContract);
        expect(type).to.deep.equal('dataContract');
    });
});
