const dotenvSafe = require('dotenv-safe');
const path = require('path');
const webpack = require('webpack');


const { parsed: envs } = dotenvSafe.config({
    path: path.resolve(__dirname, '.env'),
});



module.exports = (config) => {
    config.set({
        autoWatch: false,
        autoWatchBatchSelay: 250,
        basePath: '',
        browserDisconnectTimeout: 1000000,
        browserDisconnectTolerance: 10,
        browserNoActivityTimeout: 1000000,
        browserSocketTimeout: 100000,
        browsers: [
            'ChromeHeadless',
            'FirefoxHeadless',
            'IE',
            'Opera'
        ],
        captureTimeout: 90000,
        client: {
            mocha: {
                timeout: 650000,
            },
        },
        colors: true,
        concurrency: Infinity,
        crossOriginAttribute: true,
        customContextFile: null,
        customLaunchers: {
            FirefoxHeadless: {
                base: 'Firefox',
                flags: ['-headless'],
            },
        },
        detached: false,
        failOnEmptyTestSuite: true,
        failOnSkippedTests: true,
        failOnFailingTestSuite: true,
        files: [
            'lib/test/browser/index.js',
        ],
        frameworks: ['mocha', 'chai'],
        hostname: 'localhost',
        logLevel: config.LOG_INFO,
        path: "/",
        pingTimeout: 10000,
        plugins: [
            'karma-chai',
            'karma-chrome-launcher',
            'karma-firefox-launcher',
            "karma-ie-launcher",
            'karma-mocha',
            'karma-mocha-reporter',
            "karma-opera-launcher",
            "karma-phantomjs-launcher",
            "karma-safari-launcher",
            'karma-webpack',
        ],
        port: 9876,
        processKillTimeout: 2000,
        preprocessors: {
            'lib/test/browser/index.js': ['webpack'],
        },
        protocol: 'http:',
        reportSlowerThan: 0,
        restartOnFileChange: true,
        retryLimit: 5,
        singleRun: false,
        webpack: {
            mode: 'development',
            resolve: {
                fallback: {
                    fs: false,
                    http: require.resolve('stream-http'),
                    https: require.resolve('https-browserify'),
                    crypto: require.resolve('crypto-browserify'),
                    buffer: require.resolve('buffer/'),
                    assert: require.resolve('assert-browserify'),
                    stream: require.resolve('stream-browserify'),
                    path: require.resolve('path-browserify'),
                    url: require.resolve('url/'),
                    os: require.resolve('os-browserify/browser'),
                    zlib: require.resolve('browserify-zlib'),
                },
                alias: {
                    process: 'process/browser',
                },
            },
            plugins: [
                new webpack.ProvidePlugin({
                    Buffer: ['buffer', 'Buffer'],
                    process: 'process/browser'
                }),
                new webpack.EnvironmentPlugin(Object.keys(envs)),
            ],
        },
    })
}