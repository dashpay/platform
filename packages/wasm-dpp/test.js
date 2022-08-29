const { default: load_dpp } = require('./dist');

async function main() {
    console.log("Starting test");

    let Dpp = await load_dpp();

    console.dir(Dpp);

    let { Identifier, Transaction } = Dpp;

    let buf = Uint8Array.from(Buffer.from('f1'.repeat(32), 'hex'));
    let id = new Identifier(buf);

    console.log(id.toString());
    console.log(id.type);

    const buf2 = Buffer.from(id);
    console.log('buf2:', buf2);

    try {
        let id2 = new Identifier(Uint8Array.from([0,0]));
    } catch (e) {
        console.error(e);
    }

    let tx = new Transaction();

    console.log("tx version: ", tx.version());

}

main().catch(console.error);
