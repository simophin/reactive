import javax.inject.Inject

plugins {
    id("com.android.application") version "9.3.3"
}

android {
    namespace = "com.reactive.flexdemo"
    compileSdk = 36

    defaultConfig {
        applicationId = "com.reactive.flexdemo"
        minSdk = 26
        targetSdk = 36
        versionCode = 1
        versionName = "0.1.0"
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

dependencies {
    implementation("com.reactive:android-lib:0.1.0")
}

/** Builds the `flex-demo` cdylib with cargo-ndk into a jniLibs layout. */
abstract class CargoNdkBuild : DefaultTask() {
    @get:Input
    abstract val abis: ListProperty<String>

    @get:Input
    abstract val release: Property<Boolean>

    @get:Internal
    abstract val workspaceDir: DirectoryProperty

    @get:OutputDirectory
    abstract val outputDir: DirectoryProperty

    @get:Inject
    abstract val execOps: ExecOperations

    init {
        // Cargo does its own change tracking.
        outputs.upToDateWhen { false }
    }

    @TaskAction
    fun build() {
        val args = mutableListOf("cargo", "ndk", "--platform", "26")
        abis.get().forEach { args += listOf("-t", it) }
        args += listOf("-o", outputDir.get().asFile.absolutePath, "build", "-p", "flex-demo", "--lib")
        if (release.get()) args += "--release"
        execOps.exec {
            workingDir = workspaceDir.get().asFile
            commandLine(args)
        }
    }
}

androidComponents {
    onVariants { variant ->
        val name = variant.name.replaceFirstChar { it.uppercase() }
        val cargo = tasks.register<CargoNdkBuild>("cargoNdkBuild$name") {
            // e.g. -Preactive.abis=arm64-v8a,x86_64
            abis.set(
                providers.gradleProperty("reactive.abis").orElse("arm64-v8a")
                    .map { it.split(",").map(String::trim) }
            )
            release.set(variant.buildType == "release")
            workspaceDir.set(layout.projectDirectory.dir("../../.."))
            outputDir.set(layout.buildDirectory.dir("rustJniLibs/${variant.name}"))
        }
        variant.sources.jniLibs?.addGeneratedSourceDirectory(cargo, CargoNdkBuild::outputDir)
    }
}
