import QtQuick 2.15
import QtQuick.Layouts 1.15
import RinUI

FluentPage {
    objectName: "QueuePage"
    title: "Queue"
    wrapperWidth: 960

    Frame {
        Layout.fillWidth: true
        Layout.preferredHeight: 190
        color: Theme.currentTheme.colors.cardSecondaryColor
        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 22
            spacing: 8
            Text { typography: Typography.Caption; color: Theme.currentTheme.colors.primaryColor; text: "QUEUE / LOW ENERGY" }
            Text { typography: Typography.Title; text: "Three things worth keeping nearby" }
            Text {
                Layout.fillWidth: true
                typography: Typography.Body
                color: Theme.currentTheme.colors.textSecondaryColor
                text: "The queue is deliberately a short list, not a dashboard. RinUI keeps the action and the state on the same plane."
                wrapMode: Text.WordWrap
            }
        }
    }

    SettingCard {
        Layout.fillWidth: true
        title: "The quiet before the blue hour"
        description: "Episode 04  ·  23 minutes  ·  ready to preview"
        icon.name: "ic_fluent_play_circle_20_regular"
        actionIcon.name: "ic_fluent_play_20_filled"
        clickable: true
    }
    SettingCard {
        Layout.fillWidth: true
        title: "A little room for tomorrow"
        description: "Saved note  ·  3 minutes to revisit"
        icon.name: "ic_fluent_note_20_regular"
        actionIcon.name: "ic_fluent_more_horizontal_20_regular"
        clickable: true
    }
    SettingCard {
        Layout.fillWidth: true
        title: "Metadata review"
        description: "3 subjects  ·  paused until you ask"
        icon.name: "ic_fluent_tag_20_regular"
        actionIcon.name: "ic_fluent_chevron_right_20_regular"
        clickable: true
    }
}
