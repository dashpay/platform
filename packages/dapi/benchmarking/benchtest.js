const {cl, misc} = require('khal');
const memwatch = require('memwatch-next');
const Benchtest = {
    getConsumption:function(){
        let snapshot = new memwatch.HeapDiff();
        let diff = snapshot.end();
        let memuse = process.memoryUsage();
        return {
            memory:{
                os:{
                    freemem:misc.formatByteSize(os.freemem()),
                    total:misc.formatByteSize(os.totalmem())
                },
                nodeUsage:{
                    rss:misc.formatByteSize(memuse.rss),
                    heapTotal:misc.formatByteSize(memuse.heapTotal),
                    heapUsed:misc.formatByteSize(memuse.heapUsed),
                    external:misc.formatByteSize(memuse.external),
                }
            },
            snapshotDiff:diff,
        };
    },
    startInternal:function(){
        cl(`Benchmarking - Internal test (non-connected to the DAPI network)`);
        cl(this.getConsumption());
        cl(`--- Networking`);
        cl(`Starting 1000 internal nodes`);
        cl(`Feeding the network with heavy data`);
        cl(`Feeding the network with small data`);
        cl(`Feeding the network with regular data`);
        cl(`--- REST Server`);
        cl(`--- AUTHService`);
    }
};
Benchtest.startInternal();