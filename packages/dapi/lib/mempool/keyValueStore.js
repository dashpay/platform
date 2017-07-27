class KeyValueStore extends require('./mempoolBase') {
    constructor(port, namespace = 'dapinet') {
        super(port)
        this.kvs = this.orbitdb.kvstore(namespace)
        this.init();
    }

    init(key = 'message') {

        this.kvs.events.on('ready', () => {
            console.log("ready: " + this.kvs.get(key))
        })

        this.kvs.events.on('synced', () => {
            console.log("synced: " + this.kvs.get(key))
        })
    }

    writeValue(value, key = 'message') {
        this.kvs.set(key, value)
            .then(() => {
                console.log(this.kvs.get(key))
            })
    }

    getValue(key = 'message') {
        return this.kvs.get(key)
    }

    contains(key) {
        return this.kvs.get(key) !== 'undefined'
    }

}

module.exports = KeyValueStore
