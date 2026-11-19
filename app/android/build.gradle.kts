allprojects {
    repositories {
        google()
        mavenCentral()
    }
}

val newBuildDir: Directory =
    rootProject.layout.buildDirectory
        .dir("../../build")
        .get()
rootProject.layout.buildDirectory.value(newBuildDir)

subprojects {
    val newSubprojectBuildDir: Directory = newBuildDir.dir(project.name)
    project.layout.buildDirectory.value(newSubprojectBuildDir)

    plugins.withId("com.android.library") {
        extensions.configure<com.android.build.api.variant.LibraryAndroidComponentsExtension> {
            finalizeDsl { ext ->
                if ((ext.compileSdk ?: 0) < 36) {
                    ext.compileSdk = 36
                }
            }
        }
    }
}
subprojects {
    project.evaluationDependsOn(":app")
}

// Third-party plugin lint (e.g. google_sign_in_android) crashes AGP lint on JDK 17
// (CommentDetector calls List.removeLast(), a JDK 21 API). Skip lintVital tasks.
gradle.taskGraph.whenReady {
    allTasks.filter { it.name.startsWith("lintVital") }.forEach { it.enabled = false }
}

tasks.register<Delete>("clean") {
    delete(rootProject.layout.buildDirectory)
}
