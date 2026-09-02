import QtQuick 2.15
import QtQuick.Layouts 1.15
import RinUI

FluentPage {
    objectName: "LibraryPage"
    title: "Library"
    wrapperWidth: 960

    Frame {
        Layout.fillWidth: true
        Layout.preferredHeight: 168
        color: Theme.currentTheme.colors.cardSecondaryColor
        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 22
            spacing: 8
            Text { typography: Typography.Caption; color: Theme.currentTheme.colors.primaryColor; text: "LIBRARY / INDEXED STATE" }
            Text { typography: Typography.Title; text: "A shelf with a point of view" }
            Text {
                Layout.fillWidth: true
                typography: Typography.Body
                color: Theme.currentTheme.colors.textSecondaryColor
                text: "RinUI's navigation, cards, and list surfaces keep the desktop hierarchy visible without turning every item into a web card."
                wrapMode: Text.WordWrap
            }
        }
    }

    Text { typography: Typography.Subtitle; text: "Collection pulse" }

    SettingCard {
        Layout.fillWidth: true
        title: "26 local episodes ready"
        description: "The strongest state is expressed as a row: icon, explanation, status, direction."
        icon.name: "ic_fluent_video_clip_20_regular"
        actionIcon.name: "ic_fluent_chevron_right_20_regular"
        clickable: true
    }
    SettingCard {
        Layout.fillWidth: true
        title: "3 subjects need metadata review"
        description: "A secondary action can stay in the page stack until the user asks for it."
        icon.name: "ic_fluent_warning_20_regular"
        actionIcon.name: "ic_fluent_chevron_right_20_regular"
        clickable: true
    }
    SettingCard {
        Layout.fillWidth: true
        title: "Last scan  ·  2 minutes ago"
        description: "The prototype intentionally leaves persistence and backend calls out of scope."
        icon.name: "ic_fluent_arrow_sync_20_regular"
        actionIcon.name: "ic_fluent_more_horizontal_20_regular"
        clickable: true
    }
}
