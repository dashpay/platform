package org.dashfoundation.example

import android.app.Application
import org.dashfoundation.example.di.AppContainer

/**
 * Application entry — owns the [AppContainer] (← `SwiftExampleAppApp.init`,
 * which creates the ModelContainer and state objects before the first view
 * renders).
 */
class ExampleApplication : Application() {

    lateinit var container: AppContainer
        private set

    override fun onCreate() {
        super.onCreate()
        container = AppContainer(this)
    }
}
