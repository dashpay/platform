const axios = require('axios');
const explorerGet = require('./common/ExplorerHelper').explorerGet;

exports.getStatus = function() {

    return new Promise(function(resolve, reject) {
        explorerGet(`/status`)
            .then(data => {
                resolve(data);
            })
            .catch(error => {
                reject(`An error was triggered while fetching candidate - signaling and removing from list:` + error);
            })
    });
}



exports.getMNlist = function() {

    return new Promise(async function(resolve, reject) {

        SDK.Discover.getInsightCandidateURI()
            .then(getInsightURI => {
                let url = 'http://test-insight.dev.dash.org/insight-api-dash/status?q=getMNlist';
                let protocol = url.split(':')[0];
                let path = url.split(/(\/+)/)[4];

                console.log('./Explorer/API/getStatus', `replace with this when PR done: ${getInsightURI}/status?q=getMNlist`); //!!!! replace with this when PR done

                _fetch({ type: "GET", url: url }, function(err, data) {
                    if (err) {
                        console.error(`An error was triggered while fetching candidate - signaling and removing from list`);
                        //TODO: Do this thing!
                        resolve(false);
                    }
                    var parsed = data.slice(1, 10).map((mn) => {
                        return {
                            protocol: protocol,
                            path: `/${path}`,
                            base: mn.IP.split(':')[0],
                            port: mn.IP.split(':')[1],
                        }
                    })
                    resolve(parsed);
                });

            })
    });
}

