plugins {
    id("com.android.application")
}

android {
    namespace = "com.naya.hod"
    compileSdk {
        version = release(37)
    }

    defaultConfig {
        applicationId = "com.naya.hod"
        minSdk = 26
        targetSdk = 37
        versionCode = 1
        versionName = "0.1.0"
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_11
        targetCompatibility = JavaVersion.VERSION_11
    }
}

dependencies {
    testImplementation("junit:junit:4.13.2")
}
