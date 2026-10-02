plugins {
    `java-gradle-plugin`
    kotlin("jvm") version "2.4.20"
}

group = "com.reactive"
version = "0.1.0"

repositories {
    google()
    mavenCentral()
}

dependencies {
    compileOnly("com.android.tools.build:gradle:9.3.3")

    testImplementation(gradleTestKit())
    testImplementation("com.android.tools.build:gradle:9.3.3")
    testImplementation("org.junit.jupiter:junit-jupiter-api:5.10.2")
    testRuntimeOnly("org.junit.jupiter:junit-jupiter-engine:5.10.2")
}

gradlePlugin {
    plugins {
        create("reactivePlugin") {
            id = "com.reactive"
            implementationClass = "com.reactive.gradle.ReactivePlugin"
        }
    }
}

tasks.test {
    useJUnitPlatform()
}
