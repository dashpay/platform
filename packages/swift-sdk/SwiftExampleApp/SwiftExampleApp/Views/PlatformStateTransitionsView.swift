import SwiftUI

struct PlatformStateTransitionsView: View {
    var body: some View {
        StateTransitionsView()
    }
}

struct PlatformStateTransitionsView_Previews: PreviewProvider {
    static var previews: some View {
        NavigationView {
            PlatformStateTransitionsView()
                .environmentObject(UnifiedAppState())
        }
    }
}