'use strict';
const requesterJSON = require('./requesterJSON');
const Fetcher = {
    _fetch: function (opts, cb) {
        let _GET = function (opts, cb) {
            requesterJSON
                .get(opts.url)
                .then(function (r) {
                    cb(null, r);
                })
                .catch(function(e){
                    console.error('Error while fetching :', e);
                    cb(e, null);
                });
        };
        let _POST = function (opts, cb) {
            requesterJSON
                .post({host:opts.host, port:opts.port, auth:opts.auth}, opts.data)
                .then(function (r) {
                    cb(null, r);
                })
                .catch(function(e){
                    console.error('Error while fetching :', e);
                    cb(e, null);
                });
        };
        var self = this;
        if (opts.type) {
            if (opts.type == 'GET') {
                _GET(opts, cb);
            }
            if(opts.type=='POST'){
                _POST(opts, cb);
            }
        } else {
            cb('missing parameter', null);
        }
    },
};
module.exports = Fetcher;