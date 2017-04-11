const has = require('../../util/has.js');
const {uuid}=require('khal');

exports.create = function() {
    let self = this;
    return async function (_u) {
        return new Promise(function (resolve, reject) {
            let res = {error: null, result: 'success'};
            if(
                _u &&
                has(_u,'username') &&
                has(_u,'password') &&
                has(_u,'email')
            ){
                let msg = {
                    type:"user",
                    action:"create",
                    user:_u,
                    _reqId:uuid.generate.v4()
                };
                self.emitter.once(msg._reqId, function(data){
                    if(data.hasOwnProperty('error') && data.error==null){
                        return resolve(data.message);
                    }else{
                        return resolve(data.message);
                    }
                });
                self.socket.send(JSON.stringify(msg));
            }
            else{
                res.error = '100 - Missing Params';
                res.result = 'Missing User';
                return resolve(res);
            }
        });
    }
}