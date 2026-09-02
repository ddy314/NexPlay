import QtQuick 2.15
import RinUI

FluentWindow {
    id: window

    visible: true
    width: 1440
    height: 900
    minimumWidth: 980
    minimumHeight: 680
    title: "NexPlay"

    navigationView.navExpandWidth: 224
    navigationItems: [
        {
            title: "Focus",
            page: Qt.resolvedUrl("Home.qml"),
            icon: "ic_fluent_sparkle_20_regular",
            position: Position.Top
        },
        {
            title: "Library",
            page: Qt.resolvedUrl("Library.qml"),
            icon: "ic_fluent_library_20_regular",
            position: Position.Top
        },
        {
            title: "Queue",
            page: Qt.resolvedUrl("Queue.qml"),
            icon: "ic_fluent_play_circle_20_regular",
            position: Position.Top
        },
        {
            title: "Settings",
            page: Qt.resolvedUrl("Settings.qml"),
            icon: "ic_fluent_settings_20_regular",
            position: Position.Bottom
        }
    ]
    defaultPage: Qt.resolvedUrl("Home.qml")

    titleBarArea: TextField {
        width: 320
        placeholderText: "Search your collection"
    }
}
