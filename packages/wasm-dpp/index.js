// Note that a dynamic `import` statement here is required due to
// webpack/webpack#6615, but in theory `import { greet } from './pkg';`

// TODO: IMPORTANT NOTICE! THIS IS WORKS ONLY IF BUILT WITH npm run build:node
const dpp_module = require('./pkg');
const assert = require('assert');
const Dpp = require('@dashevo/dpp');
const getIdenityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');
const convertBuffersToArrays = require('@dashevo/dpp/lib/util/convertBuffersToArrays');
const { default: Ajv } = require('ajv/dist/2020');
const addByteArrayKeyword = require('@dashevo/dpp/lib/ajv/keywords/byteArray/addByteArrayKeyword');
const schmea = require("@dashevo/dpp/schema/identity/identity.json");

const { IdentityFacade, DashPlatformProtocol } = dpp_module;

const identityFacade = new IdentityFacade();

const validationResult = identityFacade.validate({
            "protocolVersion":1,
            "id": [198, 23, 40, 120, 58, 93, 0, 165, 27, 49, 4, 117, 107, 204,  67, 46, 164, 216, 230, 135, 201, 92, 31, 155, 62, 131, 211, 177, 139, 175, 163, 237],
            "publicKeys": [
                {"id":0,"type":0,"purpose":0,"securityLevel":0,"data":"AuryIuMtRrl/VviQuyLD1l4nmxi9ogPzC9LT7tdpo0di","readOnly":false},
                {"id":1,"type":0,"purpose":1,"securityLevel":3,"data":"A8AK95PYMVX5VQKzOhcVQRCUbc9pyg3RiL7jttEMDU+L","readOnly":false}
            ],
            "balance":10,
            "revision":0
    }
);

assert(validationResult.isValid());

console.log('is valid first?', validationResult.isValid())
console.log(validationResult.errorsText());

const validationResult2 = identityFacade.validate({
        "protocolVersion": 1,
        "id": [198, 23, 40, 120, 58, 93, 0, 165, 27, 49, 4, 117, 107, 204,  67, 46, 164, 216, 230, 135, 201, 92, 31, 155, 62, 131, 211, 177, 139, 175, 163, 237],
        "publicKeys": [
            {"id":0,"type":0,"purpose":0,"securityLevel":0,"data":"AuryIuMtRrl/VviQuyLD1l4nmxi9ogPzC9LT7tdpo0di","readOnly":false},
            {"id":1,"type":0,"purpose":1,"securityLevel":3,"data":"A8AK95PYMVX5VQKzOhcVQRCUbc9pyg3RiL7jttEMDU+L","readOnly":false}
        ],
        "balance": "this is not a correct balance",
        "revision":0
    }
);

assert(validationResult2.isValid() === false);

console.log('is valid second?', validationResult2.isValid())
console.log(validationResult2.errorsText());

// rust
//     .then(dpp => {
//         const { Identifier, Identity } = dpp;
//         const identifier = Identifier.fromString("EDCuAy8AXqAh56eFRkKRKb79SC35csP3W9VPe1UMaz87")
//         console.log(identifier.toString());
//         const buf = identifier.toBuffer();
//         console.log(identifier.toBuffer());
//         console.log(Array.from(identifier.toBuffer()).map(u8 => u8.toString(16)).join(''))
//         const id2 = new Identifier(identifier.toBuffer());
//         console.log('id2', id2.toString());
//         const id3 = Identifier.from("EDCuAy8AXqAh56eFRkKRKb79SC35csP3W9VPe1UMaz87")
//         console.log('id3', id3.toString());
//         console.log('buf:', buf);
//         const id4 = Identifier.from(buf);
//         console.log('id4', id4.toString());
//
//         let i = Identity.new();
//         console.log("the originl object", i);
//         console.log("the identity", i.toString());
//         console.log("the identity", i.toObject());
//         console.log("the public keys", i.getPublicKeys());
//     })
//     .catch(console.error);

async function bench() {
    let rustDpp = new DashPlatformProtocol();
    let jsDpp = new Dpp();

    let fixtue = getIdenityFixture().toObject();

    await jsDpp.initialize();

    let runs = 1000;

    const start_js = Date.now();

    console.time("js");
    for (let i = 0; i<runs; i++) {
        let result = jsDpp.identity.validate(fixtue);
        if (!result.isValid()) {
            throw new Error(result.errors[0]);
        }
    }
    console.timeEnd("js");

    const end_js = Date.now();

    const time_js = end_js - start_js;

    console.log(`${time_js * 1000 / runs} s per 1000`);

    let start_rust = Date.now();

    console.time("rust");
    for (let i = 0; i<runs; i++) {
        let result = identityFacade.validate(convertBuffersToArrays(fixtue));

        if (!result.isValid()) {
            throw new Error(result.errorsText()[0]);
        }

        result.free();
    }
    console.timeEnd("rust");

    const end_rust = Date.now();

    const time_rust = end_rust - start_rust;

    console.log(`${time_rust / runs} ms per run`);

    console.log(`Rust is ${time_js / time_rust} times faster`);

    const ajv = new Ajv();
    addByteArrayKeyword(ajv);
    const validate = ajv.compile(schmea);

    console.time("ajv");
    for (let i = 0; i<runs; i++) {
        validate(fixtue);
        // let result = jsDpp.identity.validate(fixtue);
        // if (!result.isValid()) {
        //     throw new Error(result.errors[0]);
        // }
    }
    console.timeEnd("pks");

    console.time("pks");
    for (let i = 0; i<runs; i++) {
        jsDpp.identity.validatePublicKeys(fixtue.publicKeys);
        // let result = jsDpp.identity.validate(fixtue);
        // if (!result.isValid()) {
        //     throw new Error(result.errors[0]);
        // }
    }
    console.timeEnd("pks");

}

bench().catch(console.error)
