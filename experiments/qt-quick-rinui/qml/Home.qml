import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import RinUI

FluentPage {
    id: page
    objectName: "FocusPage"
    title: "Focus"
    wrapperWidth: 1120
    horizontalPadding: 42
    contentSpacing: 18

    property bool previewRunning: false
    property bool syncEnabled: true

    RowLayout {
        Layout.fillWidth: true
        spacing: 10

        Text {
            typography: Typography.Subtitle
            text: "Tonight's focus"
        }

        Text {
            Layout.fillWidth: true
            typography: Typography.Body
            color: Theme.currentTheme.colors.textSecondaryColor
            text: "A quiet queue, one deliberate choice."
        }

        PillButton {
            text: "All"
            checked: true
            width: 54
        }
        PillButton {
            text: "Short"
            width: 70
        }
        ToolButton {
            icon.name: "ic_fluent_more_horizontal_20_regular"
            size: 18
            width: 40
            height: 40
            ToolTip.visible: hovered
            ToolTip.text: "More focus options"
        }
    }

    Frame {
        id: hero
        Layout.fillWidth: true
        Layout.preferredHeight: 312
        clip: true
        color: Theme.currentTheme.colors.cardColor

        RowLayout {
            anchors.fill: parent
            spacing: 0

            ColumnLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.margins: 26
                spacing: 10

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 8
                    Text {
                        typography: Typography.Caption
                        color: Theme.currentTheme.colors.primaryColor
                        text: "NEXT UP  /  EPISODE 04"
                    }
                    InfoBadge {
                        dot: true
                        severity: Severity.Success
                    }
                }

                Text {
                    Layout.fillWidth: true
                    typography: Typography.TitleLarge
                    text: "The quiet before\nthe blue hour"
                }

                Text {
                    Layout.fillWidth: true
                    Layout.maximumWidth: 500
                    typography: Typography.Body
                    color: Theme.currentTheme.colors.textSecondaryColor
                    text: "A 23-minute pick for the end of the day — atmospheric, unhurried, and already 68% familiar."
                    wrapMode: Text.WordWrap
                }

                Item { Layout.fillHeight: true }

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 12
                    Button {
                        highlighted: true
                        icon.name: page.previewRunning ? "ic_fluent_pause_20_filled" : "ic_fluent_play_20_filled"
                        text: page.previewRunning ? "Pause preview" : "Preview motion"
                        onClicked: page.previewRunning = !page.previewRunning
                    }
                    Button {
                        flat: true
                        icon.name: "ic_fluent_add_20_regular"
                        text: "Add to queue"
                    }
                }
            }

            Frame {
                Layout.preferredWidth: 360
                Layout.fillHeight: true
                clip: true
                frameless: true
                hoverable: false

                Image {
                    anchors.fill: parent
                    source: "qrc:/nexplay-qt/aurora-cover.svg"
                    fillMode: Image.PreserveAspectCrop
                    asynchronous: true
                    opacity: page.previewRunning ? 1 : 0.88
                }

                Rectangle {
                    anchors.fill: parent
                    color: Theme.currentTheme.colors.backgroundSmokeColor
                    opacity: 0.28
                }

                ColumnLayout {
                    anchors.left: parent.left
                    anchors.bottom: parent.bottom
                    anchors.leftMargin: 20
                    anchors.bottomMargin: 20
                    spacing: 2

                    Text {
                        typography: Typography.Caption
                        color: "#e5f2ff"
                        text: "BLUE HOUR / 00:16:02"
                    }
                    Text {
                        typography: Typography.BodyStrong
                        color: "#ffffff"
                        text: page.previewRunning ? "Ambient preview" : "Ready when you are"
                    }
                }

                Behavior on opacity {
                    NumberAnimation { duration: Utils.appearanceSpeed; easing.type: Easing.OutCubic }
                }
            }
        }
    }

    RowLayout {
        Layout.fillWidth: true
        spacing: 12

        Frame {
            Layout.fillWidth: true
            Layout.preferredHeight: 124
            color: Theme.currentTheme.colors.cardSecondaryColor
            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 18
                spacing: 4
                Text { typography: Typography.Caption; color: Theme.currentTheme.colors.textSecondaryColor; text: "WATCHED THIS WEEK" }
                Text { typography: Typography.Title; text: "06h 42m" }
                Text { typography: Typography.Caption; color: Theme.currentTheme.colors.systemSuccessColor; text: "+18% from last week" }
            }
        }
        Frame {
            Layout.fillWidth: true
            Layout.preferredHeight: 124
            color: Theme.currentTheme.colors.cardSecondaryColor
            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 18
                spacing: 4
                Text { typography: Typography.Caption; color: Theme.currentTheme.colors.textSecondaryColor; text: "QUEUE ENERGY" }
                RowLayout {
                    spacing: 8
                    Text { typography: Typography.Title; text: "Low" }
                    InfoBadge { text: "calm"; severity: Severity.Info }
                }
                ProgressBar { Layout.fillWidth: true; value: 0.34 }
            }
        }
        Frame {
            Layout.fillWidth: true
            Layout.preferredHeight: 124
            color: Theme.currentTheme.colors.cardSecondaryColor
            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 18
                spacing: 4
                Text { typography: Typography.Caption; color: Theme.currentTheme.colors.textSecondaryColor; text: "LIBRARY SIGNAL" }
                Text { typography: Typography.Title; text: "94%" }
                Text { typography: Typography.Caption; color: Theme.currentTheme.colors.textSecondaryColor; text: "metadata in agreement" }
            }
        }
    }

    RowLayout {
        Layout.fillWidth: true
        spacing: 18

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 10
            Text { typography: Typography.Subtitle; text: "A considered next step" }
            SettingCard {
                Layout.fillWidth: true
                title: "Continue from where the room gets quiet"
                description: "Episode 04  ·  16 minutes remaining  ·  local"
                icon.name: "ic_fluent_history_20_regular"
                actionIcon.name: "ic_fluent_play_circle_20_filled"
                clickable: true
                onClicked: page.previewRunning = true
            }
            SettingCard {
                Layout.fillWidth: true
                title: "One saved note is waiting"
                description: "The visual language prototype is intentionally static."
                icon.name: "ic_fluent_note_20_regular"
                clickable: true
            }
        }

        Frame {
            Layout.preferredWidth: 290
            Layout.preferredHeight: 178
            color: Theme.currentTheme.colors.controlFillSecondaryColor
            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 18
                spacing: 8
                Text { typography: Typography.Subtitle; text: "Quiet sync" }
                Text {
                    Layout.fillWidth: true
                    typography: Typography.Caption
                    color: Theme.currentTheme.colors.textSecondaryColor
                    text: "A native toggle with no confirmation ceremony."
                    wrapMode: Text.WordWrap
                }
                Item { Layout.fillHeight: true }
                RowLayout {
                    Layout.fillWidth: true
                    Text { Layout.fillWidth: true; typography: Typography.Body; text: page.syncEnabled ? "On" : "Off" }
                    Switch { checked: page.syncEnabled; onToggled: page.syncEnabled = checked }
                }
            }
        }
    }
}
