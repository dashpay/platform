const has = require('../../util/has.js');
const {uuid}=require('khal');

exports.search = function() {
    let self = this;
    return async function(query){
        return new Promise(function (resolve, reject) {
            let res = {error: null, result: 'success'};
            if(
                query &&
                (has(query,'username') ||
                has(query,'email'))
            ){
                let msg = {
                    type:"user",
                    action:"search",
                    query:query,
                    _reqId:uuid.generate.v4()
                };
                self.emitter.once(msg._reqId, function(data){
                    if(data.hasOwnProperty('error') && data.error==null){
                        return resolve(data);
                    }else{
                        return resolve(data.message);
                    }
                });
                self.socket.send(JSON.stringify(msg));
            }
            else{
                res.error = '100 - Missing Params';
                res.result = 'Missing Query';
                return resolve(res);
            }
        });
    }
}