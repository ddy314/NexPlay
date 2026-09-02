#include <QGuiApplication>
#include <QDebug>
#include <QQmlApplicationEngine>
#include <QQmlComponent>
#include <QQmlError>
#include <QUrl>
#include <cstdio>

int main(int argc, char *argv[])
{
    QGuiApplication app(argc, argv);
    app.setApplicationName(QStringLiteral("NexPlay Qt prototype"));
    app.setOrganizationName(QStringLiteral("NexPlay"));

    QQmlApplicationEngine engine;
    engine.addImportPath(QStringLiteral(NEXPLAY_RINUI_SOURCE_ROOT));
    QObject::connect(&engine, &QQmlApplicationEngine::warnings, [](const QList<QQmlError> &warnings) {
        for (const QQmlError &warning : warnings)
            qWarning().noquote() << warning.toString();
    });

    const QUrl entryPoint(QStringLiteral("qrc:/nexplay-qt/qml/Main.qml"));
    QObject::connect(
        &engine,
        &QQmlApplicationEngine::objectCreationFailed,
        &app,
        [entryPoint](const QUrl &url) {
            if (url == entryPoint)
                QCoreApplication::exit(1);
        },
        Qt::QueuedConnection);

    engine.load(entryPoint);
    if (engine.rootObjects().isEmpty()) {
        QQmlComponent diagnostic(&engine, entryPoint, QQmlComponent::PreferSynchronous);
        for (const QQmlError &error : diagnostic.errors())
            std::fprintf(stderr, "%s\\n", error.toString().toLocal8Bit().constData());
        return 1;
    }

    return app.exec();
}
