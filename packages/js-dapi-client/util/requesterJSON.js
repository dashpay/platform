const url = require('url');
const http = require('http');
const https = require('https');
const timeout = 10 * 1000;//60 seconde timeout (time to get the response)
const {is} = require('khal');

const requesterJSON = {
    prepareRequest: function (URL) {
        const parseURL = url.parse(URL);
        const PORT = parseURL.port;
        const protocol = parseURL.protocol;
        const hostname = parseURL.hostname;
        const path = parseURL.path;
        URL = (protocol === null) ? 'http://' + URL : URL;
        return {path:path, hostname:hostname,URL: URL, port:PORT, requester: (protocol === 'https:') ? https : http};
    },
    get: function (URL) {
        if (!URL) throw("Require URL");

        return new Promise(function (resolve, reject) {
            const prepare = requesterJSON.prepareRequest(URL);
            const _url = prepare.URL;
            const _hostname = prepare.hostname;
            const _path = prepare.path;
            const _req = prepare.requester;
            const _port = prepare.port;
            const get_options = {
                hostname:_hostname,
                path:_path,
                port:_port,
                method: 'GET',
                // headers: {
                //     "User-Agent": 'Node-Client'
                // }
            };
            let request = _req.request(get_options, function (response) {
                const statusCode = response.statusCode;
                if (statusCode === 200) {
                    response.setEncoding('utf8');
                    let rawData = '';
                    response.on('data', (chunk) => rawData += chunk);
                    response.on('end', () => {
                        try {
                            if(!is.stringified(rawData))
                                return reject('Not JSON - [GET]'+_url);
                            let parsedData = JSON.parse(rawData);
                            return resolve(parsedData);
                        } catch (e) {
                            return resolve(e.message);
                        }
                    });
                } else if (statusCode == 302 || statusCode == 301) {
                    //Redirection
                    let newURL = response.headers.location;
                    console.log("Redirect to", newURL);
                    // throw("Moved to ",newURL)
                    return resolve(requester.get(newURL));
                } else if (statusCode === 404) {
                    // throw("Unreachable domain", statusCode);
                    return resolve(statusCode);
                }
                else {
                    // throw("Got an statusCode", statusCode);
                    return resolve(statusCode);
                }
            })
                .on('error', function (e) {
                    return reject(e);
                });
            request.setTimeout(timeout, function () {
                request.abort();
                //Gateway time-out
                return resolve(504);
            })
            request.end();

        });


    },
    post: function (options, data) {
        return new Promise(function (resolve, reject) {
            const _hostname = options.host;
            const _path = options.path || '/';
            const _port = options.port || 80;
            const _req = http;
            const _requestData = JSON.stringify(data);

            const post_options = {
                hostname:_hostname,
                path:_path,
                port:_port,
                method: 'POST',
                headers: {
                    'Content-Length': _requestData.length
                }
            };
            if (options.auth) {
                post_options.auth = (options.auth);
            }
            let request = _req.request(post_options, function (response) {
                const statusCode = response.statusCode;
                if (statusCode === 200) {
                    response.setEncoding('utf8');
                    let rawData = '';
                    response.on('data', (chunk) => rawData += chunk);
                    response.on('end', () => {
                        try {
                            if(!is.stringified(rawData))
                                return reject('Not JSON - [POST]'+_hostname,_port,_path);
                            let parsedData = JSON.parse(rawData);
                            return resolve(parsedData);
                        } catch (e) {
                            return resolve(e.message);
                        }
                    });
                } else if (statusCode == 302 || statusCode == 301) {
                    //Redirection
                    let newURL = response.headers.location;
                    console.log("Redirect to", newURL);
                    // throw("Moved to ",newURL)
                    return resolve(requester.get(newURL));
                } else if (statusCode === 404) {
                    // throw("Unreachable domain", statusCode);
                    return resolve(statusCode);
                }
                else {
                    // throw("Got an statusCode", statusCode);
                    return resolve(statusCode);
                }
            })
                .on('error', function (e) {
                    return reject(e);
                });
            request.setTimeout(timeout, function () {
                request.abort();
                //Gateway time-out
                return resolve(504);
            })
            request.end(_requestData);


        });
    }
};
module.exports = requesterJSON;