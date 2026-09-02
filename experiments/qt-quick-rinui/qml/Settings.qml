import QtQuick 2.15
import QtQuick.Layouts 1.15
import RinUI

FluentPage {
    objectName: "SettingsPage"
    title: "Settings"
    wrapperWidth: 920

    Text {
        typography: Typography.Subtitle
        text: "Prototype surfaces"
    }

    SettingCard {
        Layout.fillWidth: true
        title: "Use ambient transitions"
        description: "RinUI page and control animations remain short, spatial, and easy to interrupt."
        icon.name: "ic_fluent_motion_20_regular"
        Switch { checked: true }
    }
    SettingCard {
        Layout.fillWidth: true
        title: "Accent color"
        description: "Theme color stays owned by RinUI's Theme singleton."
        icon.name: "ic_fluent_color_20_regular"
        Button { highlighted: true; text: "Aurora" }
    }
    SettingCard {
        Layout.fillWidth: true
        title: "Reduced motion"
        description: "A real product setting would map this to the animation policy."
        icon.name: "ic_fluent_accessibility_20_regular"
        Switch { checked: false }
    }
}
