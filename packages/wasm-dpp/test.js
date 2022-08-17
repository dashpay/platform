let load_dpp = require('./dist');

async function main() {
    let Dpp = await load_dpp.default();

    let { Identifier } = Dpp;

    let buf = Uint8Array.from(Buffer.from('f1'.repeat(32), 'hex'));
    let id = new Identifier(buf);

    console.log(id.toString());

    try {
        let id2 = new Identifier(Uint8Array.from([0,0]));
    } catch (e) {
        console.error(e);
    }

}

main().then(() => console.log("Finished")).catch(console.error);