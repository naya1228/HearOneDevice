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

    // 릴리스 서명 열쇠는 환경 변수로만 받음 (CI: .github/workflows/release.yml). 없으면 릴리스 APK는 서명 안 됨
    val keystore = System.getenv("HOD_KEYSTORE")
    signingConfigs {
        if (keystore != null) {
            create("release") {
                storeFile = file(keystore)
                storePassword = System.getenv("HOD_KEYSTORE_PASSWORD")
                keyAlias = System.getenv("HOD_KEY_ALIAS")
                keyPassword = System.getenv("HOD_KEY_PASSWORD")
            }
        }
    }
    buildTypes {
        release {
            if (keystore != null) signingConfig = signingConfigs.getByName("release")
        }
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
