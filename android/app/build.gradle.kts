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
        versionCode = 4
        versionName = "0.4.0"
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_11
        targetCompatibility = JavaVersion.VERSION_11
    }
}

dependencies {
    // 구글 플레이 서비스의 QR 스캐너 (카메라 권한 불필요)
    implementation("com.google.android.gms:play-services-code-scanner:16.1.0")
    testImplementation("junit:junit:4.13.2")
}
