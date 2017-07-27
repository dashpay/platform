const ipfsapi = require('ipfs-api'),
    OrbitDB = require('orbit-db'),
    util = require("util")

class MempoolBase {
    constructor(port = 5001) {
        try {
            this.orbitdb = new OrbitDB(ipfsapi('127.0.0.1', port));
        }
        catch (e) {
            console.log(`Check if ipfs deamon is running on port ${port}. Exception: ${ex}`)
        }
    }

    dump_obj(obj) {
        console.log(
            util.inspect(
                obj,
                {
                    showHidden: true,
                    depth: null,
                    maxArrayLength: null,
                    breakLength: null
                })
        )
        return null;
    }
}

module.exports = MempoolBase