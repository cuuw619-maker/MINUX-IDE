plugins {
    kotlin("jvm") version "2.2.20"
    application
}

group = "dev.minux"
version = "1.0.0"

kotlin {
    jvmToolchain(21)
}

application {
    mainClass.set("dev.minux.WorkspaceDoctorKt")
}

dependencies {
    testImplementation(kotlin("test-junit5"))
    testRuntimeOnly("org.junit.jupiter:junit-jupiter-engine:5.12.2")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher:1.12.2")
}

tasks.test {
    useJUnitPlatform()
}
