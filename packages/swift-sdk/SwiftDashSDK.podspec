Pod::Spec.new do |s|
  s.name         = 'SwiftDashSDK'
  s.version      = '0.0.1'
  s.summary      = 'Swift SDK for the Dash Platform'
  s.description  = 'Swift wrapper around DashSDKFFI providing access to Dash Platform, wallet, and SPV functionality.'
  s.homepage     = 'https://github.com/dashpay/platform'
  s.license      = { :type => 'MIT', :text => 'MIT License' }
  s.authors      = { 'Dash Core Group, Inc.' => 'contact@dash.org' }
  s.source       = { :git => 'https://github.com/dashpay/platform.git' }

  s.ios.deployment_target = '17.0'
  s.swift_versions = ['6.0', '5.10']

  s.source_files = 'Sources/SwiftDashSDK/**/*.swift'
  s.exclude_files = 'Sources/SwiftDashSDK/KeyWallet/README.md'

  s.vendored_frameworks = 'DashSDKFFI.xcframework'

  s.pod_target_xcconfig = {
    'OTHER_LDFLAGS' => '-lDashSDKFFI_combined',
  }
end
